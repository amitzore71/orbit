//! A serialized, UI-independent application runtime. SQLite waits never run on the UI thread.
mod protocol;
use focus_core::{Clock, Command, Effect, Timer, WidgetPreferences};
use focus_storage::{Loaded, Operation, Response, Worker};
pub use protocol::*;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

type EventSink = Arc<dyn Fn(RuntimeEvent) + Send + Sync>;
struct Request {
    command: AppCommand,
    reply: mpsc::SyncSender<Result<Reply>>,
    normal: bool,
}
#[derive(Clone)]
pub struct RuntimeHandle {
    sender: mpsc::SyncSender<Request>,
    observers: Arc<AtomicUsize>,
    queued: Arc<AtomicUsize>,
}
impl RuntimeHandle {
    /// Call from a blocking task, never from the native window thread.
    pub fn call(&self, command: AppCommand) -> Result<Reply> {
        let (tx, rx) = mpsc::sync_channel(1);
        let normal = !matches!(
            command,
            AppCommand::Finish
                | AppCommand::Timer {
                    action: TimerAction::Suspend | TimerAction::Resume
                }
        );
        if normal
            && self
                .queued
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    (n < 128).then_some(n + 1)
                })
                .is_err()
        {
            return Err("Orbit is busy. Please retry.".to_string().into());
        }
        if self
            .sender
            .try_send(Request {
                command,
                reply: tx,
                normal,
            })
            .is_err()
        {
            if normal {
                self.queued.fetch_sub(1, Ordering::AcqRel);
            }
            return Err("Orbit is busy. Please retry.".to_string().into());
        }
        rx.recv()
            .map_err(|_| RuntimeError::from("Orbit runtime has stopped.".to_string()))?
    }
    pub fn set_timer_observers(&self, count: usize) {
        self.observers.store(count, Ordering::Release);
    }
}

pub fn start(
    path: PathBuf,
    clock: Arc<dyn Clock + Send + Sync>,
    events: EventSink,
) -> std::result::Result<RuntimeHandle, String> {
    // Two extra slots preserve lifecycle/exit requests when normal work fills the queue.
    let (tx, rx) = mpsc::sync_channel::<Request>(130);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let observers = Arc::new(AtomicUsize::new(0));
    let observed = observers.clone();
    let queued = Arc::new(AtomicUsize::new(0));
    let waiting = queued.clone();
    thread::Builder::new()
        .name("orbit-runtime".into())
        .spawn(move || {
            let (storage, loaded) = match Worker::open(path.clone()) {
                Ok(v) => v,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            let mut runtime = Runtime::new(storage, loaded, path, clock, events);
            let _ = ready_tx.send(Ok(()));
            let mut checkpoint_at = Instant::now() + Duration::from_secs(60);
            let mut calendar_at = Instant::now() + Duration::from_secs(60);
            let mut display_at = Instant::now();
            loop {
                let now = Instant::now();
                let snap = runtime.timer.snapshot(runtime.clock.now());
                let expiry = if snap.running {
                    now + Duration::from_millis(snap.remaining_ms.max(1))
                } else {
                    now + Duration::from_secs(60)
                };
                let mut wake = expiry.min(calendar_at);
                if snap.running {
                    wake = wake.min(checkpoint_at);
                }
                if snap.running && observed.load(Ordering::Acquire) > 0 {
                    wake = wake.min(display_at);
                }
                match rx.recv_timeout(wake.saturating_duration_since(now)) {
                    Ok(request) => {
                        if request.normal {
                            waiting.fetch_sub(1, Ordering::AcqRel);
                        }
                        let finish = matches!(request.command, AppCommand::Finish);
                        let result = runtime.execute(request.command);
                        let done = finish && result.is_ok();
                        let _ = request.reply.send(result);
                        if done {
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = runtime.execute(AppCommand::Finish);
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let now = Instant::now();
                if !snap.running && runtime.timer.snapshot(runtime.clock.now()).running {
                    checkpoint_at = now + Duration::from_secs(60);
                }
                if now >= calendar_at {
                    let today = focus_storage::local_date();
                    if today != runtime.today {
                        runtime.today = today;
                        runtime.changed(&["tasks", "reports", "today"]);
                    }
                    calendar_at = now + Duration::from_secs(60);
                }
                if now >= checkpoint_at {
                    if runtime.timer.snapshot(runtime.clock.now()).running {
                        let _ = runtime.transition(Command::Checkpoint);
                    }
                    checkpoint_at = now + Duration::from_secs(60);
                }
                if now >= display_at || now >= expiry {
                    if let Err(error) = runtime.transition(Command::Tick) {
                        (runtime.events)(RuntimeEvent::Error(error));
                    }
                    display_at = now + Duration::from_secs(1);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    ready_rx.recv().map_err(|e| e.to_string())??;
    Ok(RuntimeHandle {
        sender: tx,
        observers,
        queued,
    })
}

struct Runtime {
    storage: Worker,
    timer: Timer,
    clock: Arc<dyn Clock + Send + Sync>,
    events: EventSink,
    tasks: Vec<focus_storage::Task>,
    widgets: [WidgetPreferences; 3],
    revision: u64,
    today: String,
    data_location: String,
    last_backup: Option<String>,
    recovery_message: Option<String>,
    finished: bool,
}
impl Runtime {
    fn new(
        storage: Worker,
        loaded: Loaded,
        path: PathBuf,
        clock: Arc<dyn Clock + Send + Sync>,
        events: EventSink,
    ) -> Self {
        let timer = loaded.checkpoint.map_or_else(
            || Timer::new(loaded.settings.clone()),
            |c| Timer::restore(loaded.settings.clone(), c),
        );
        let mut value=Self {storage,timer,clock,events,tasks:loaded.tasks,widgets:loaded.widgets,revision:1,today:focus_storage::local_date(),data_location:path.display().to_string(),last_backup:loaded.last_backup,recovery_message:loaded.recovered.then(||"An interrupted session was restored paused. Unsaved running time was not added.".into()),finished:false};
        value.reconcile_selection();
        value
    }
    fn changed(&mut self, domains: &[&'static str]) {
        self.revision += 1;
        (self.events)(RuntimeEvent::Changed {
            revision: self.revision,
            domains: domains.to_vec(),
        });
        (self.events)(RuntimeEvent::Timer(self.timer_snapshot()));
    }
    fn timer_snapshot(&self) -> TimerSnapshot {
        let now = self.clock.now();
        let s = self.timer.snapshot(now);
        let c = self.timer.checkpoint(now);
        TimerSnapshot {
            revision: self.revision,
            mode: s.mode,
            running: s.running,
            started: s.started,
            remaining_ms: s.remaining_ms,
            total_ms: s.total_ms,
            cycle: s.cycle,
            bound_task: c.bound_task.map(|x| x.to_string()),
            selected_task: c.selected_task.map(|x| x.to_string()),
        }
    }
    fn raw(&self, operation: Operation) -> Result<Response> {
        let retained_write = matches!(
            operation,
            Operation::Transition { .. }
                | Operation::Settings(_)
                | Operation::AddTask { .. }
                | Operation::CreateTask { .. }
                | Operation::EditTask { .. }
                | Operation::ToggleTaskDone(_)
                | Operation::DeleteTask(_)
                | Operation::RestoreTask(_)
                | Operation::PlanTask { .. }
                | Operation::MoveTask { .. }
                | Operation::Widget { .. }
                | Operation::Flush
        );
        let (tx, rx) = mpsc::sync_channel(1);
        self.storage
            .submit(operation, move |r| {
                let _ = tx.send(r);
            })
            .map_err(RuntimeError::from)?;
        rx.recv()
            .map_err(|_| RuntimeError::from("Storage has stopped.".to_string()))?
            .map_err(|message| RuntimeError {
                message,
                accepted: retained_write && self.storage.is_blocked(),
            })
    }
    fn store(&mut self, operation: Operation) -> Result<Response> {
        match self.raw(operation) {
            Ok(response) => Ok(response),
            Err(error) => {
                if self.storage.is_blocked() {
                    let now = self.clock.now();
                    let effects = self.timer.dispatch(Command::Pause, now);
                    let _ = self
                        .storage
                        .preserve_pause(effects, self.timer.checkpoint(now));
                    self.changed(&["storage", "timer"]);
                }
                Err(error)
            }
        }
    }
    fn transition(&mut self, command: Command) -> Result<()> {
        if self.storage.is_blocked() && matches!(command, Command::Suspend | Command::Resume) {
            self.timer.dispatch(command, self.clock.now());
            (self.events)(RuntimeEvent::Timer(self.timer_snapshot()));
            return Ok(());
        }
        if self.storage.is_blocked()
            && !matches!(command, Command::Suspend | Command::Resume | Command::Tick)
        {
            return Err("Saving is paused. Retry saving before resuming."
                .to_string()
                .into());
        }
        let previous = self.timer.clone();
        let now = self.clock.now();
        let effects = self.timer.dispatch(command, now);
        let tasks_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::SessionEnded { .. }));
        let completed = effects
            .iter()
            .filter_map(|e| {
                if let Effect::Completed(m) = e {
                    Some(*m)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !matches!(command, Command::Tick) || !effects.is_empty() {
            let result = self.store(Operation::Transition {
                effects,
                checkpoint: self.timer.checkpoint(now),
            });
            if let Err(e) = result {
                if !e.accepted {
                    self.timer = previous;
                }
                return Err(e);
            }
            if tasks_changed {
                self.refresh_tasks()?;
                self.changed(&["timer", "tasks", "today", "reports"]);
            } else if matches!(command, Command::Checkpoint) {
                self.changed(&["timer", "today"]);
            } else {
                self.changed(&["timer", "today", "reports"]);
            }
        } else {
            (self.events)(RuntimeEvent::Timer(self.timer_snapshot()));
        }
        for mode in completed {
            (self.events)(RuntimeEvent::Completed(mode));
        }
        Ok(())
    }
    fn refresh_tasks(&mut self) -> Result<()> {
        if let Response::Tasks(tasks) = self.raw(Operation::Tasks)? {
            self.tasks = tasks;
            self.reconcile_selection();
        }
        Ok(())
    }
    fn reconcile_selection(&mut self) {
        let c = self.timer.checkpoint(self.clock.now());
        if c.selected_task.is_some_and(|id| {
            !self
                .tasks
                .iter()
                .any(|t| t.id == id && !t.done && !t.deleted)
        }) {
            self.timer
                .dispatch(Command::SelectTask(None), self.clock.now());
        }
    }
    fn snapshot(&self) -> Result<AppSnapshot> {
        let (today_ms, today_completed) = match self.raw(Operation::Today)? {
            Response::Today(v) => (v.active_ms, v.completed),
            _ => (0, 0),
        };
        Ok(AppSnapshot {
            revision: self.revision,
            timer: self.timer_snapshot(),
            tasks: self.tasks.iter().map(TaskDto::from).collect(),
            settings: self.timer.settings.clone(),
            widgets: self.widgets.clone(),
            today: self.today.clone(),
            today_ms,
            today_completed,
            storage_blocked: self.storage.is_blocked(),
            data_location: self.data_location.clone(),
            last_backup: self.last_backup.clone(),
            recovery_message: self.recovery_message.clone(),
        })
    }
    fn id(&self, id: &str, active: bool) -> Result<i64> {
        let id = id
            .parse::<i64>()
            .map_err(|_| RuntimeError::from("Invalid task ID.".to_string()))?;
        if !self
            .tasks
            .iter()
            .any(|t| t.id == id && (!active || (!t.done && !t.deleted)))
        {
            return Err("Task is no longer available.".to_string().into());
        }
        Ok(id)
    }
    fn report_operation(query: &ReportQuery) -> Operation {
        if query.start.is_empty() && query.end.is_empty() {
            Operation::Report {
                period: query.period.storage(),
                offset: query.offset.clamp(-1200, 1200),
            }
        } else {
            Operation::ReportRange {
                start: query.start.clone(),
                end: query.end.clone(),
            }
        }
    }
    fn execute(&mut self, command: AppCommand) -> Result<Reply> {
        if self.finished {
            return Err("Orbit has exited.".to_string().into());
        }
        match command {
            AppCommand::Snapshot => return self.snapshot().map(|s| Reply::Snapshot(Box::new(s))),
            AppCommand::Timer { action } => self.transition(match action {
                TimerAction::Start => Command::Start,
                TimerAction::Pause => Command::Pause,
                TimerAction::Toggle => Command::Toggle,
                TimerAction::Skip => Command::Skip,
                TimerAction::Reset => Command::Reset,
                TimerAction::Suspend => Command::Suspend,
                TimerAction::Resume => Command::Resume,
                TimerAction::Checkpoint => Command::Checkpoint,
            })?,
            AppCommand::SelectMode { mode } => self.transition(Command::SelectMode(mode))?,
            AppCommand::SelectTask { id } => {
                let id = id.as_deref().map(|v| self.id(v, true)).transpose()?;
                self.transition(Command::SelectTask(id))?;
            }
            AppCommand::StartTask { id } => {
                let id = self.id(&id, true)?;
                self.transition(Command::StartTask(id))?;
            }
            AppCommand::CreateTask {
                title,
                estimate,
                today,
                start,
            } => {
                if let Response::TaskCreated(task) = self.store(Operation::CreateTask {
                    title,
                    estimate,
                    today,
                })? {
                    let id = task.id;
                    self.tasks.push(task);
                    self.changed(&["tasks"]);
                    if start {
                        self.transition(Command::StartTask(id))?;
                    }
                    return Ok(Reply::Created(id.to_string()));
                }
            }
            AppCommand::EditTask {
                id,
                title,
                estimate,
            } => {
                let id = self.id(&id, false)?;
                self.store(Operation::EditTask {
                    id,
                    title,
                    estimate,
                })?;
                self.refresh_tasks()?;
                self.changed(&["tasks"]);
            }
            AppCommand::Task { id, action } => {
                let id = self.id(&id, false)?;
                let operation = match action {
                    TaskAction::ToggleDone => Operation::ToggleTaskDone(id),
                    TaskAction::Delete => Operation::DeleteTask(id),
                    TaskAction::Restore => Operation::RestoreTask(id),
                    TaskAction::Plan => Operation::PlanTask { id, today: true },
                    TaskAction::Unplan => Operation::PlanTask { id, today: false },
                    TaskAction::Up => Operation::MoveTask { id, direction: -1 },
                    TaskAction::Down => Operation::MoveTask { id, direction: 1 },
                };
                self.store(operation)?;
                self.refresh_tasks()?;
                self.changed(&["tasks", "timer", "reports"]);
            }
            AppCommand::SaveSettings { mut settings } => {
                settings.normalize_appearance();
                settings.validate().map_err(RuntimeError::from)?;
                self.store(Operation::Settings(settings.clone()))?;
                self.timer.settings = settings.clone();
                let s = self.timer.snapshot(self.clock.now());
                if !s.started {
                    self.timer
                        .dispatch(Command::SelectMode(s.mode), self.clock.now());
                }
                (self.events)(RuntimeEvent::Settings(settings));
                self.changed(&["settings", "timer"]);
            }
            AppCommand::Widget { index, preferences } => {
                if index >= 3 || !(0..=2).contains(&preferences.layer) {
                    return Err("Invalid widget preferences.".to_string().into());
                }
                self.store(Operation::Widget {
                    index,
                    preferences: preferences.clone(),
                })?;
                self.widgets[index] = preferences;
                self.changed(&["widgets"]);
            }
            AppCommand::MoveWidget { index, x, y } => {
                let Some(mut p) = self.widgets.get(index).cloned() else {
                    return Err("Invalid widget.".to_string().into());
                };
                p.x = Some(x);
                p.y = Some(y);
                self.store(Operation::Widget {
                    index,
                    preferences: p.clone(),
                })?;
                self.widgets[index] = p;
                self.changed(&["widgets"]);
            }
            AppCommand::Report { query } => {
                self.transition(Command::Checkpoint)?;
                if let Response::Report(r) = self.raw(Self::report_operation(&query))? {
                    return Ok(Reply::Report(r.into()));
                }
            }
            AppCommand::SessionDetails { id, query } => {
                let task_id = id
                    .as_deref()
                    .map(|v| {
                        v.parse::<i64>()
                            .map_err(|_| RuntimeError::from("Invalid task ID.".to_string()))
                    })
                    .transpose()?;
                if let Response::SessionDetails(rows) = self.raw(Operation::SessionDetails {
                    task_id,
                    period: query.period.storage(),
                    offset: query.offset,
                    start: query.start,
                    end: query.end,
                })? {
                    return Ok(Reply::Details(rows));
                }
            }
            AppCommand::Backup { path } => {
                self.transition(Command::Pause)?;
                if let Response::BackedUp(p) = self.store(Operation::Backup(path.into()))? {
                    self.last_backup = Some(p.display().to_string());
                    self.changed(&["data"]);
                    return Ok(Reply::File(p.display().to_string()));
                }
            }
            AppCommand::Restore { path } => {
                self.transition(Command::Pause)?;
                if let Response::Restored(loaded) = self.store(Operation::Restore(path.into()))? {
                    self.timer = loaded.checkpoint.map_or_else(
                        || Timer::new(loaded.settings.clone()),
                        |c| Timer::restore(loaded.settings.clone(), c),
                    );
                    self.tasks = loaded.tasks;
                    self.widgets = loaded.widgets;
                    self.last_backup = loaded.last_backup;
                    self.reconcile_selection();
                    (self.events)(RuntimeEvent::Settings(self.timer.settings.clone()));
                    self.changed(&["tasks", "settings", "widgets", "reports", "data", "timer"]);
                }
            }
            AppCommand::Export { path, query } => {
                self.transition(Command::Checkpoint)?;
                if let Response::Exported(p) = self.raw(Operation::ExportReport {
                    path: path.into(),
                    period: query.period.storage(),
                    offset: query.offset,
                    start: query.start,
                    end: query.end,
                })? {
                    return Ok(Reply::File(p.display().to_string()));
                }
            }
            AppCommand::RetrySaving => {
                self.store(Operation::Flush)?;
                if let Response::Reloaded(loaded) = self.raw(Operation::Reload)? {
                    self.timer = loaded.checkpoint.map_or_else(
                        || Timer::new(loaded.settings.clone()),
                        |c| Timer::restore(loaded.settings.clone(), c),
                    );
                    self.tasks = loaded.tasks;
                    self.widgets = loaded.widgets;
                    self.last_backup = loaded.last_backup;
                    self.reconcile_selection();
                }
                (self.events)(RuntimeEvent::Settings(self.timer.settings.clone()));
                self.changed(&[
                    "storage", "tasks", "widgets", "settings", "timer", "today", "reports",
                ]);
            }
            AppCommand::Finish => {
                let now = self.clock.now();
                let previous = self.timer.clone();
                let effects = self.timer.dispatch(Command::Pause, now);
                let (tx, rx) = mpsc::sync_channel(1);
                if let Err(e) = self
                    .storage
                    .finish(effects, self.timer.checkpoint(now), move |r| {
                        let _ = tx.send(r);
                    })
                {
                    self.timer = previous;
                    return Err(e.into());
                }
                rx.recv()
                    .map_err(|_| {
                        RuntimeError::from("Storage stopped during final save.".to_string())
                    })?
                    .map_err(|message| RuntimeError {
                        message,
                        accepted: true,
                    })?;
                self.finished = true;
            }
        }
        Ok(Reply::Saved(self.revision))
    }
}

#[cfg(test)]
mod tests;
