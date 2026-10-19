//! SQLite lives on a single blocking worker; callers receive bounded, asynchronous responses.
use chrono::{Datelike, Days, Local, Months, NaiveDate, TimeZone};
use focus_core::{Checkpoint, Effect, Settings, WidgetPreferences};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

const QUEUE_CAPACITY: usize = 128;
const MAX_PENDING: usize = QUEUE_CAPACITY + 2;

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub estimate: u32,
    pub completed: u32,
    pub done: bool,
}
#[derive(Clone, Copy, Debug)]
pub enum Period {
    Day,
    Week,
    Month,
}
#[derive(Clone, Debug, Default)]
pub struct DayReport {
    pub label: String,
    pub active_ms: u64,
    pub completed: u32,
}
#[derive(Clone, Debug, Default)]
pub struct TaskReport {
    pub task_id: Option<i64>,
    pub title: String,
    pub deleted: bool,
    pub active_ms: u64,
    pub completed: u32,
}
impl TaskReport {
    fn new(task_id: Option<i64>, title: Option<String>, deleted: bool) -> Self {
        Self {
            task_id,
            title: title.unwrap_or_else(|| match task_id {
                Some(id) => format!("Deleted task #{id}"),
                None => "Without a task".into(),
            }),
            deleted: task_id.is_some() && deleted,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub title: String,
    pub days: Vec<DayReport>,
    pub tasks: Vec<TaskReport>,
    pub active_ms: u64,
    pub completed: u32,
    pub partial_ms: u64,
}
#[derive(Clone, Debug, Default)]
pub struct FocusSummary {
    pub active_ms: u64,
    pub completed: u32,
}
pub struct Loaded {
    pub settings: Settings,
    pub checkpoint: Option<Checkpoint>,
    pub widgets: [WidgetPreferences; 3],
    pub tasks: Vec<Task>,
    pub recovered: bool,
}
#[derive(Clone)]
pub enum Operation {
    Transition {
        effects: Vec<Effect>,
        checkpoint: Checkpoint,
    },
    Settings(Settings),
    AddTask {
        title: String,
        estimate: u32,
    },
    EditTask {
        id: i64,
        title: String,
        estimate: u32,
        done: bool,
    },
    DeleteTask(i64),
    Widget {
        index: usize,
        preferences: WidgetPreferences,
    },
    Tasks,
    Report {
        period: Period,
        offset: i32,
    },
    Today,
    Flush,
}
pub enum Response {
    Saved,
    Flushed,
    Tasks(Vec<Task>),
    Report(Report),
    Today(FocusSummary),
}
type Job = Box<dyn FnOnce(&mut Store) + Send>;
pub struct Worker {
    sender: Option<mpsc::SyncSender<Job>>,
    thread: Option<thread::JoinHandle<()>>,
    blocked: Arc<AtomicBool>,
    recovery_queued: Arc<AtomicBool>,
}
impl Worker {
    pub fn open(path: PathBuf) -> Result<(Self, Loaded)> {
        let (sender, receiver) = mpsc::sync_channel::<Job>(QUEUE_CAPACITY);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("orbit-storage".into())
            .spawn(move || {
                let mut store = match Store::open(path) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let loaded = store.load();
                let valid = loaded.is_ok();
                let _ = ready_tx.send(loaded);
                if valid {
                    for job in receiver {
                        job(&mut store);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok((
            Self {
                sender: Some(sender),
                thread: Some(thread),
                blocked: Arc::new(AtomicBool::new(false)),
                recovery_queued: Arc::new(AtomicBool::new(false)),
            },
            ready_rx.recv().map_err(|e| e.to_string())??,
        ))
    }
    pub fn submit(
        &self,
        operation: Operation,
        callback: impl FnOnce(Result<Response>) + Send + 'static,
    ) -> Result<()> {
        validate_operation(&operation)?;
        let write = !matches!(
            operation,
            Operation::Tasks | Operation::Report { .. } | Operation::Today | Operation::Flush
        );
        if write && self.is_blocked() {
            return Err(
                "Saving is paused after a database error. Retry a timer control to retry saving."
                    .into(),
            );
        }
        self.enqueue(operation, callback, false)
    }
    pub fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::Acquire)
    }
    /// One final paused checkpoint is retained after a write failure; further activity is blocked.
    pub fn preserve_pause(&self, effects: Vec<Effect>, checkpoint: Checkpoint) -> Result<()> {
        if checkpoint.running || self.recovery_queued.swap(true, Ordering::AcqRel) {
            return Err("A paused recovery checkpoint is already pending.".into());
        }
        let operation = Operation::Transition {
            effects,
            checkpoint,
        };
        validate_operation(&operation)?;
        self.enqueue(operation, |_| {}, true)
    }
    fn enqueue(
        &self,
        operation: Operation,
        callback: impl FnOnce(Result<Response>) + Send + 'static,
        wait: bool,
    ) -> Result<()> {
        let blocked = self.blocked.clone();
        let recovery_queued = self.recovery_queued.clone();
        let flush = matches!(operation, Operation::Flush);
        let write = !matches!(
            operation,
            Operation::Tasks | Operation::Report { .. } | Operation::Today
        );
        let job: Job = Box::new(move |store| {
            let result = store.process(operation);
            if write && result.is_err() {
                blocked.store(true, Ordering::Release);
            }
            if flush && result.is_ok() {
                recovery_queued.store(false, Ordering::Release);
                blocked.store(false, Ordering::Release);
            }
            callback(result);
        });
        let sender = self.sender.as_ref().ok_or("Storage has stopped.")?;
        if wait {
            return sender.send(job).map_err(|e| e.to_string());
        }
        sender
            .try_send(job)
            .map_err(|_| "Storage is busy or unavailable. Please retry.".into())
    }
    pub fn flush(&self) -> Result<()> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.enqueue(
            Operation::Flush,
            move |result| {
                let _ = tx.send(result);
            },
            true,
        )?;
        rx.recv().map_err(|e| e.to_string())??;
        self.recovery_queued.store(false, Ordering::Release);
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn validate_operation(operation: &Operation) -> Result<()> {
    match operation {
        Operation::AddTask { title, estimate }
        | Operation::EditTask {
            title, estimate, ..
        } => {
            valid_task(title, *estimate)?;
        }
        Operation::Settings(settings) => settings.validate()?,
        Operation::Widget { index, .. } if *index >= 3 => return Err("Invalid widget.".into()),
        Operation::Transition { effects, .. } if effects.len() > 8 => {
            return Err("Too many timer effects in one transition.".into());
        }
        _ => {}
    }
    Ok(())
}

struct Store {
    db: Connection,
    pending: VecDeque<Operation>,
}
impl Store {
    fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        Self::connection(Connection::open(path).map_err(|e| e.to_string())?)
    }
    fn connection(db: Connection) -> Result<Self> {
        db.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA cache_size=-1024; PRAGMA mmap_size=0; PRAGMA synchronous=FULL;")
            .map_err(|e| e.to_string())?;
        let version: u32 = db
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > 2 {
            return Err(
                "This database belongs to a newer version of Orbit. Update the app to open it."
                    .into(),
            );
        }
        if version == 0 {
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE tasks(id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, estimate INTEGER NOT NULL, done INTEGER NOT NULL DEFAULT 0, deleted INTEGER NOT NULL DEFAULT 0);
                CREATE TABLE sessions(id INTEGER PRIMARY KEY, task_id INTEGER, completed INTEGER NOT NULL, ended_ms INTEGER NOT NULL);
                CREATE TABLE intervals(id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL, task_id INTEGER, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, active_ms INTEGER NOT NULL);
                CREATE INDEX intervals_range ON intervals(start_ms, end_ms);
                CREATE INDEX sessions_range ON sessions(ended_ms);
                CREATE INDEX sessions_task ON sessions(task_id, completed);
                PRAGMA user_version=1; COMMIT;") .map_err(|e| e.to_string())?;
        }
        if version < 2 {
            db.execute_batch("BEGIN IMMEDIATE; CREATE INDEX intervals_end ON intervals(end_ms); PRAGMA user_version=2; COMMIT;").map_err(|e| e.to_string())?;
        }
        Ok(Self {
            db,
            pending: VecDeque::new(),
        })
    }
    fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let value: Option<String> = self
            .db
            .query_row("SELECT value FROM meta WHERE key=?", [key], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        value
            .map(|v| serde_json::from_str(&v).map_err(|e| format!("Invalid saved {key}: {e}")))
            .transpose()
    }
    fn put<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        self.db.execute("INSERT INTO meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,json]).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn load(&self) -> Result<Loaded> {
        let settings: Settings = self.get("settings")?.unwrap_or_default();
        settings.validate()?;
        let checkpoint: Option<Checkpoint> = self.get("timer")?;
        let recovered = checkpoint.as_ref().is_some_and(|c| c.running);
        let widgets: [WidgetPreferences; 3] = self.get("widgets")?.unwrap_or_else(|| {
            let mut values: [WidgetPreferences; 3] = Default::default();
            values[2].layer = 2;
            values
        });
        Ok(Loaded {
            settings,
            checkpoint,
            widgets,
            tasks: self.tasks()?,
            recovered,
        })
    }
    fn tasks(&self) -> Result<Vec<Task>> {
        Self::tasks_in(&self.db)
    }
    fn tasks_in(db: &Connection) -> Result<Vec<Task>> {
        let mut query = db.prepare("SELECT t.id,t.title,t.estimate,t.done,(SELECT COUNT(*) FROM sessions s WHERE s.task_id=t.id AND s.completed=1) FROM tasks t WHERE deleted=0 ORDER BY done,id").map_err(|e| e.to_string())?;
        query
            .query_map([], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    estimate: row.get(2)?,
                    done: row.get(3)?,
                    completed: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    fn process(&mut self, operation: Operation) -> Result<Response> {
        validate_operation(&operation)?;
        let flush = matches!(operation, Operation::Flush);
        if matches!(
            operation,
            Operation::Tasks | Operation::Report { .. } | Operation::Today
        ) {
            return self.execute(operation);
        }
        if !flush {
            if self.pending.len() >= MAX_PENDING {
                return Err("Storage retry capacity reached; activity must remain paused.".into());
            }
            self.pending.push_back(operation);
        }
        let mut response = Response::Saved;
        // Keep failed transactions intact for a later retry, including an explicit exit/flush.
        while let Some(operation) = self.pending.front().cloned() {
            response = self.execute(operation)?;
            self.pending.pop_front();
        }
        if flush {
            self.execute(Operation::Flush)
        } else {
            Ok(response)
        }
    }
    fn execute(&mut self, operation: Operation) -> Result<Response> {
        let sql = |e: rusqlite::Error| e.to_string();
        match operation {
            Operation::Transition {
                effects,
                checkpoint,
            } => {
                let tx = self.db.transaction().map_err(sql)?;
                for effect in effects {
                    match effect {
                        Effect::FocusInterval {
                            session_id,
                            task_id,
                            start_utc_ms,
                            end_utc_ms,
                            active_ms,
                        } => {
                            tx.execute("INSERT INTO intervals(session_id,task_id,start_ms,end_ms,active_ms) VALUES(?1,?2,?3,?4,?5)", params![session_id as i64,task_id,start_utc_ms,end_utc_ms,active_ms as i64]).map_err(sql)?;
                        }
                        Effect::SessionEnded {
                            session_id,
                            task_id,
                            completed,
                            ended_utc_ms,
                        } => {
                            tx.execute("INSERT INTO sessions(id,task_id,completed,ended_ms) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO NOTHING", params![session_id as i64,task_id,completed,ended_utc_ms]).map_err(sql)?;
                        }
                        Effect::Completed(_) => {}
                    }
                }
                let json = serde_json::to_string(&checkpoint).map_err(|e| e.to_string())?;
                tx.execute("INSERT INTO meta(key,value) VALUES('timer',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [json]).map_err(sql)?;
                tx.commit().map_err(sql)?;
                Ok(Response::Saved)
            }
            Operation::Settings(settings) => {
                settings.validate()?;
                self.put("settings", &settings)?;
                Ok(Response::Saved)
            }
            Operation::AddTask { title, estimate } => {
                let title = valid_task(&title, estimate)?;
                let tx = self.db.transaction().map_err(sql)?;
                tx.execute(
                    "INSERT INTO tasks(title,estimate) VALUES(?1,?2)",
                    params![title, estimate],
                )
                .map_err(sql)?;
                let tasks = Self::tasks_in(&tx)?;
                tx.commit().map_err(sql)?;
                Ok(Response::Tasks(tasks))
            }
            Operation::EditTask {
                id,
                title,
                estimate,
                done,
            } => {
                let title = valid_task(&title, estimate)?;
                let tx = self.db.transaction().map_err(sql)?;
                tx.execute(
                    "UPDATE tasks SET title=?2,estimate=?3,done=?4 WHERE id=?1 AND deleted=0",
                    params![id, title, estimate, done],
                )
                .map_err(sql)?;
                let tasks = Self::tasks_in(&tx)?;
                tx.commit().map_err(sql)?;
                Ok(Response::Tasks(tasks))
            }
            Operation::DeleteTask(id) => {
                let tx = self.db.transaction().map_err(sql)?;
                tx.execute("UPDATE tasks SET deleted=1 WHERE id=?", [id])
                    .map_err(sql)?;
                let tasks = Self::tasks_in(&tx)?;
                tx.commit().map_err(sql)?;
                Ok(Response::Tasks(tasks))
            }
            Operation::Widget { index, preferences } => {
                if index >= 3 {
                    return Err("Invalid widget.".into());
                }
                let mut widgets: [WidgetPreferences; 3] =
                    self.get("widgets")?.unwrap_or_else(|| {
                        let mut values: [WidgetPreferences; 3] = Default::default();
                        values[2].layer = 2;
                        values
                    });
                widgets[index] = preferences;
                self.put("widgets", &widgets)?;
                Ok(Response::Saved)
            }
            Operation::Tasks => Ok(Response::Tasks(self.tasks()?)),
            Operation::Report { period, offset } => {
                Ok(Response::Report(self.report(period, offset)?))
            }
            Operation::Today => {
                let today = Local::now().date_naive();
                Ok(Response::Today(self.summary_between(
                    local_midnight(today)?,
                    local_midnight(today.succ_opt().ok_or("Date overflow")?)?,
                )?))
            }
            Operation::Flush => {
                self.db
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(sql)?;
                Ok(Response::Flushed)
            }
        }
    }
    fn report(&self, period: Period, offset: i32) -> Result<Report> {
        let (start, end) = date_range(Local::now().date_naive(), period, offset)?;
        self.report_between(start, end)
    }
    fn summary_between(&self, begin: i64, finish: i64) -> Result<FocusSummary> {
        let mut statement = self.db.prepare("SELECT start_ms,end_ms,active_ms FROM intervals INDEXED BY intervals_end WHERE end_ms>?1 AND start_ms<?2").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![begin, finish], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)? as u64,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut active_ms = 0;
        for row in rows {
            let (a, b, active) = row.map_err(|e| e.to_string())?;
            if b > a {
                let overlap = b.min(finish).saturating_sub(a.max(begin)).max(0) as u64;
                active_ms += (u128::from(active) * u128::from(overlap) / (b - a) as u128) as u64;
            }
        }
        let completed = self
            .db
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE completed=1 AND ended_ms>=?1 AND ended_ms<?2",
                params![begin, finish],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(FocusSummary {
            active_ms,
            completed,
        })
    }
    fn report_between(&self, start: NaiveDate, end: NaiveDate) -> Result<Report> {
        if start >= end {
            return Err("Empty report range.".into());
        }
        let mut dates = Vec::new();
        let mut day = start;
        while day < end {
            dates.push(day);
            day = day.succ_opt().ok_or("Date overflow")?;
        }
        let boundaries: Vec<i64> = dates
            .iter()
            .copied()
            .chain(std::iter::once(end))
            .map(local_midnight)
            .collect::<Result<_>>()?;
        let mut days: Vec<DayReport> = dates
            .iter()
            .map(|d| DayReport {
                label: d
                    .format(if dates.len() == 7 { "%a" } else { "%d" })
                    .to_string(),
                ..DayReport::default()
            })
            .collect();
        let begin = boundaries[0];
        let finish = *boundaries.last().ok_or("Empty report")?;
        let mut tasks = BTreeMap::new();
        let mut statement = self.db.prepare("SELECT i.start_ms,i.end_ms,i.active_ms,COALESCE(s.completed,0),i.task_id FROM intervals i INDEXED BY intervals_end LEFT JOIN sessions s ON s.id=i.session_id WHERE i.end_ms>?1 AND i.start_ms<?2").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![begin, finish], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)? as u64,
                    r.get::<_, bool>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut partial_ms = 0u64;
        for row in rows {
            let (a, b, active, completed, task_id) = row.map_err(|e| e.to_string())?;
            if b <= a {
                continue;
            }
            let first = boundaries
                .partition_point(|boundary| *boundary <= a)
                .saturating_sub(1);
            let mut task_ms = 0;
            for (i, bucket) in days.iter_mut().enumerate().skip(first) {
                if boundaries[i] >= b {
                    break;
                }
                let overlap = b
                    .min(boundaries[i + 1])
                    .saturating_sub(a.max(boundaries[i]))
                    .max(0) as u64;
                let credited = (u128::from(active) * u128::from(overlap) / (b - a) as u128) as u64;
                bucket.active_ms += credited;
                task_ms += credited;
                if !completed {
                    partial_ms += credited;
                }
            }
            if task_ms > 0 {
                tasks
                    .entry(task_id)
                    .or_insert_with(|| TaskReport {
                        task_id,
                        ..TaskReport::default()
                    })
                    .active_ms += task_ms;
            }
        }
        for (i, bucket) in days.iter_mut().enumerate() {
            bucket.completed = self.db.query_row("SELECT COUNT(*) FROM sessions WHERE completed=1 AND ended_ms>=?1 AND ended_ms<?2", params![boundaries[i],boundaries[i+1]], |r| r.get(0)).map_err(|e| e.to_string())?;
        }
        let mut statement = self.db.prepare("SELECT task_id,COUNT(*) FROM sessions WHERE completed=1 AND ended_ms>=?1 AND ended_ms<?2 GROUP BY task_id").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![begin, finish], |r| {
                Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, u32>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (task_id, completed) = row.map_err(|e| e.to_string())?;
            tasks
                .entry(task_id)
                .or_insert_with(|| TaskReport {
                    task_id,
                    ..TaskReport::default()
                })
                .completed = completed;
        }
        // Resolve titles once per distinct task instead of allocating one per interval.
        let mut titles = self
            .db
            .prepare("SELECT title,deleted FROM tasks WHERE id=?1")
            .map_err(|e| e.to_string())?;
        for task in tasks.values_mut() {
            let saved: Option<(String, bool)> = titles
                .query_row([task.task_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .optional()
                .map_err(|e| e.to_string())?;
            let (title, deleted) =
                saved.map_or((None, true), |(title, deleted)| (Some(title), deleted));
            let metadata = TaskReport::new(task.task_id, title, deleted);
            task.title = metadata.title;
            task.deleted = metadata.deleted;
        }
        let mut tasks: Vec<_> = tasks.into_values().collect();
        tasks.sort_by(|a, b| {
            b.active_ms
                .cmp(&a.active_ms)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.task_id.cmp(&b.task_id))
        });
        Ok(Report {
            title: if dates.len() == 1 {
                start.format("%A, %d %B").to_string()
            } else {
                format!(
                    "{} – {}",
                    start.format("%d %b %Y"),
                    end.pred_opt().unwrap().format("%d %b %Y")
                )
            },
            active_ms: days.iter().map(|d| d.active_ms).sum(),
            completed: days.iter().map(|d| d.completed).sum(),
            days,
            tasks,
            partial_ms,
        })
    }
}

fn valid_task(title: &str, estimate: u32) -> Result<&str> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 240 {
        return Err("Task title must contain 1–240 characters.".into());
    }
    if !(1..=99).contains(&estimate) {
        return Err("Estimate must be 1–99 pomodoros.".into());
    }
    Ok(title)
}
fn local_midnight(date: NaiveDate) -> Result<i64> {
    // Some time zones skip midnight. Choose the first valid local hour, earliest on folds.
    for hour in 0..24 {
        if let Some(time) = Local
            .from_local_datetime(&date.and_hms_opt(hour, 0, 0).unwrap())
            .earliest()
        {
            return Ok(time.timestamp_millis());
        }
    }
    Err("Calendar date does not exist in the local time zone.".into())
}
fn date_range(today: NaiveDate, period: Period, offset: i32) -> Result<(NaiveDate, NaiveDate)> {
    if offset.unsigned_abs() > 1200 {
        return Err("Report range is too far away.".into());
    }
    let shift = |date: NaiveDate, days: i64| {
        if days >= 0 {
            date.checked_add_days(Days::new(days as u64))
        } else {
            date.checked_sub_days(Days::new(days.unsigned_abs()))
        }
    };
    let (start, end) = match period {
        Period::Day => {
            let start = shift(today, i64::from(offset));
            (start, start.and_then(|d| d.succ_opt()))
        }
        Period::Week => {
            let monday = shift(today, -i64::from(today.weekday().num_days_from_monday()))
                .ok_or("Date overflow")?;
            let start = shift(monday, i64::from(offset) * 7);
            (start, start.and_then(|d| shift(d, 7)))
        }
        Period::Month => {
            let first = today.with_day(1).unwrap();
            let months = Months::new(offset.unsigned_abs());
            let start = if offset >= 0 {
                first.checked_add_months(months)
            } else {
                first.checked_sub_months(months)
            };
            (
                start,
                start.and_then(|d| d.checked_add_months(Months::new(1))),
            )
        }
    };
    Ok((start.ok_or("Date overflow")?, end.ok_or("Date overflow")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::{Command, TimePoint, Timer};
    fn memory() -> Store {
        Store::connection(Connection::open_in_memory().unwrap()).unwrap()
    }
    #[test]
    #[ignore = "Opt-in report timing with 100,000 intervals and 500 tasks"]
    fn report_analytics_benchmark() {
        let mut store = memory();
        let day = Local::now().date_naive();
        let begin = day.checked_sub_days(Days::new(30)).unwrap();
        let end = day.succ_opt().unwrap();
        {
            let tx = store.db.transaction().unwrap();
            for id in 1..=500 {
                tx.execute("INSERT INTO tasks(id,title,estimate) VALUES(?1,?2,3)", params![id, format!("Task {id}: Review project notes 日本語 हिंदी and prepare the next iteration")]).unwrap();
            }
            let mut interval = tx.prepare("INSERT INTO intervals(session_id,task_id,start_ms,end_ms,active_ms) VALUES(?1,?2,?3,?4,30000)").unwrap();
            let mut session = tx
                .prepare("INSERT INTO sessions(id,task_id,completed,ended_ms) VALUES(?1,?2,?3,?4)")
                .unwrap();
            for index in 0..100_000 {
                let date = day.checked_sub_days(Days::new(index % 31)).unwrap();
                let start = local_midnight(date).unwrap() + (index / 31) as i64 * 20_000;
                let id = index as i64 + 1;
                let task = id % 500 + 1;
                interval
                    .execute(params![id, task, start, start + 30_000])
                    .unwrap();
                session
                    .execute(params![id, task, index % 2 == 0, start + 30_000])
                    .unwrap();
            }
            drop(interval);
            drop(session);
            tx.commit().unwrap();
        }
        let measure = |start, end| {
            let mut samples = Vec::new();
            for _ in 0..11 {
                let clock = std::time::Instant::now();
                let report = store.report_between(start, end).unwrap();
                assert_eq!(
                    report.tasks.iter().map(|t| t.active_ms).sum::<u64>(),
                    report.active_ms
                );
                samples.push(clock.elapsed().as_secs_f64() * 1000.0);
            }
            samples.remove(0);
            samples.sort_by(f64::total_cmp);
            samples[samples.len() / 2]
        };
        let expected = store.report_between(day, end).unwrap();
        let mut summaries = Vec::new();
        for _ in 0..11 {
            let clock = std::time::Instant::now();
            let summary = store
                .summary_between(local_midnight(day).unwrap(), local_midnight(end).unwrap())
                .unwrap();
            assert_eq!(
                (summary.active_ms, summary.completed),
                (expected.active_ms, expected.completed)
            );
            summaries.push(clock.elapsed().as_secs_f64() * 1000.0);
        }
        summaries.remove(0);
        summaries.sort_by(f64::total_cmp);
        eprintln!(
            "{}",
            serde_json::json!({"intervals":100000,"tasks":500,"dayReportMedianMs":measure(day,end),"monthReportMedianMs":measure(begin,end),"todaySummaryMedianMs":summaries[summaries.len()/2]})
        );
    }
    #[test]
    fn persistent_failure_has_a_bounded_retry_queue_and_keeps_accepted_writes() {
        let mut store = memory();
        store.db.execute_batch("CREATE TRIGGER deny_task BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
        for index in 0..20_000 {
            assert!(
                store
                    .process(Operation::AddTask {
                        title: format!("Task {index}"),
                        estimate: 1
                    })
                    .is_err()
            );
            assert!(store.pending.len() <= MAX_PENDING);
        }
        assert_eq!(store.pending.len(), MAX_PENDING);
        store.db.execute_batch("DROP TRIGGER deny_task;").unwrap();
        store.process(Operation::Flush).unwrap();
        assert_eq!(store.tasks().unwrap().len(), MAX_PENDING);
        assert!(store.pending.is_empty());
    }
    #[test]
    fn a_failed_task_snapshot_rolls_back_the_mutation_before_retry() {
        let mut store = memory();
        store
            .db
            .execute(
                "INSERT INTO tasks(title,estimate) VALUES('Malformed old row',-1)",
                [],
            )
            .unwrap();
        assert!(
            store
                .process(Operation::AddTask {
                    title: "Only once".into(),
                    estimate: 1
                })
                .is_err()
        );
        assert_eq!(
            store
                .db
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        store.db.execute("UPDATE tasks SET estimate=1", []).unwrap();
        store.process(Operation::Flush).unwrap();
        assert_eq!(store.tasks().unwrap().len(), 2);
    }
    #[test]
    fn report_plan_seeks_recent_history_and_joins_completion() {
        let store = memory();
        let mut query = store.db.prepare("EXPLAIN QUERY PLAN SELECT i.start_ms,s.completed FROM intervals i INDEXED BY intervals_end LEFT JOIN sessions s ON s.id=i.session_id WHERE i.end_ms>?1 AND i.start_ms<?2").unwrap();
        let plan: Vec<String> = query
            .query_map(params![1, 2], |row| row.get(3))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            plan.iter()
                .any(|line| line.contains("intervals_end") && line.contains("end_ms>")),
            "{plan:?}"
        );
        assert!(
            plan.iter().any(|line| line.contains("INTEGER PRIMARY KEY")),
            "{plan:?}"
        );
    }
    #[test]
    fn worker_failure_closes_write_gate_and_flush_reopens_it() {
        let path = std::env::temp_dir().join(format!(
            "still-worker-{}-{}.db",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap()
        ));
        let (worker, _) = Worker::open(path.clone()).unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        worker.sender.as_ref().unwrap().send(Box::new(move |store| { store.db.execute_batch("CREATE TRIGGER deny_task BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap(); tx.send(()).unwrap(); })).unwrap();
        rx.recv().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        worker
            .submit(
                Operation::AddTask {
                    title: "Retained".into(),
                    estimate: 1,
                },
                move |result| tx.send(result.is_err()).unwrap(),
            )
            .unwrap();
        assert!(rx.recv().unwrap());
        assert!(worker.is_blocked());
        for _ in 0..10_000 {
            assert!(
                worker
                    .submit(
                        Operation::AddTask {
                            title: "Rejected".into(),
                            estimate: 1
                        },
                        |_| {}
                    )
                    .is_err()
            );
        }
        worker
            .sender
            .as_ref()
            .unwrap()
            .send(Box::new(|store| {
                store.db.execute_batch("DROP TRIGGER deny_task;").unwrap()
            }))
            .unwrap();
        worker.flush().unwrap();
        assert!(!worker.is_blocked());
        drop(worker); // joins the worker and closes SQLite before the file is reopened/deleted
        let db = Connection::open(&path).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn failed_writes_retry_atomically_before_exit() {
        let mut store = memory();
        store.db.execute_batch("CREATE TRIGGER deny_task BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'temporary failure'); END;").unwrap();
        assert!(
            store
                .process(Operation::AddTask {
                    title: "Keep this task".into(),
                    estimate: 2
                })
                .is_err()
        );
        assert!(store.tasks().unwrap().is_empty());
        assert!(store.process(Operation::Flush).is_err());
        store.db.execute_batch("DROP TRIGGER deny_task;").unwrap();
        store.process(Operation::Flush).unwrap();
        assert_eq!(store.tasks().unwrap().len(), 1);
        store.process(Operation::Flush).unwrap();
        assert_eq!(store.tasks().unwrap().len(), 1);
    }
    #[test]
    fn invalid_input_does_not_poison_retry_queue() {
        let mut store = memory();
        assert!(
            store
                .process(Operation::AddTask {
                    title: " ".into(),
                    estimate: 0
                })
                .is_err()
        );
        store
            .process(Operation::AddTask {
                title: "Valid".into(),
                estimate: 1,
            })
            .unwrap();
        store.process(Operation::Flush).unwrap();
        assert_eq!(store.tasks().unwrap().len(), 1);
    }
    #[test]
    fn rejects_future_database_versions_without_replacing_data() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA user_version=99;").unwrap();
        assert!(Store::connection(db).is_err());
    }
    #[test]
    fn reports_query_long_histories_without_loading_all_sessions() {
        let store = memory();
        let day = Local::now().date_naive();
        let midnight = local_midnight(day).unwrap();
        store.db.execute_batch("BEGIN;").unwrap();
        {
            let mut insert=store.db.prepare("INSERT INTO intervals(session_id,start_ms,end_ms,active_ms) VALUES(1,?1,?2,1000)").unwrap();
            for index in 0..10000 {
                let start = midnight + index * 1000;
                insert.execute(params![start, start + 1000]).unwrap();
            }
        }
        store.db.execute_batch("COMMIT;").unwrap();
        let report = store.report_between(day, day.succ_opt().unwrap()).unwrap();
        assert_eq!(report.active_ms, 10_000_000);
        assert_eq!(report.partial_ms, 10_000_000);
    }
    #[test]
    fn migrations_and_unicode_task_edits_preserve_history() {
        let mut s = memory();
        s.execute(Operation::AddTask {
            title: "Read — 日本語 हिंदी".into(),
            estimate: 2,
        })
        .unwrap();
        let t = s.tasks().unwrap().remove(0);
        assert_eq!(t.estimate, 2);
        s.execute(Operation::EditTask {
            id: t.id,
            title: "Revised".into(),
            estimate: 3,
            done: true,
        })
        .unwrap();
        assert!(s.tasks().unwrap()[0].done);
        s.execute(Operation::DeleteTask(t.id)).unwrap();
        assert!(s.tasks().unwrap().is_empty());
        assert_eq!(
            s.db.pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn transition_is_atomic_and_restores_running_marker() {
        let mut s = memory();
        let mut timer = Timer::new(Settings::default());
        let now = TimePoint {
            awake_ms: 0,
            utc_ms: 0,
        };
        timer.dispatch(Command::Start, now);
        s.execute(Operation::Transition {
            effects: vec![],
            checkpoint: timer.checkpoint(now),
        })
        .unwrap();
        let loaded = s.load().unwrap();
        assert!(loaded.recovered);
        assert!(loaded.checkpoint.unwrap().running);
    }
    #[test]
    fn reports_split_midnight_and_keep_partial_time_separate() {
        let s = memory();
        let d = Local::now().date_naive();
        let midnight = local_midnight(d).unwrap();
        s.db.execute(
            "INSERT INTO intervals(session_id,start_ms,end_ms,active_ms) VALUES(1,?1,?2,120000)",
            params![midnight - 60000, midnight + 60000],
        )
        .unwrap();
        let r = s.report_between(d, d.succ_opt().unwrap()).unwrap();
        assert_eq!(r.active_ms, 60000);
        assert_eq!(r.partial_ms, 60000);
        let summary = s
            .summary_between(midnight, local_midnight(d.succ_opt().unwrap()).unwrap())
            .unwrap();
        assert_eq!(
            (summary.active_ms, summary.completed),
            (r.active_ms, r.completed)
        );
        s.db.execute(
            "INSERT INTO sessions(id,completed,ended_ms) VALUES(1,1,?1)",
            [midnight + 60000],
        )
        .unwrap();
        let r = s.report_between(d, d.succ_opt().unwrap()).unwrap();
        assert_eq!(r.completed, 1);
        assert_eq!(r.partial_ms, 0);
        let summary = s
            .summary_between(midnight, local_midnight(d.succ_opt().unwrap()).unwrap())
            .unwrap();
        assert_eq!(
            (summary.active_ms, summary.completed),
            (r.active_ms, r.completed)
        );
    }
    #[test]
    fn task_reports_keep_distinct_tasks_deleted_history_and_unassigned_time() {
        let mut store = memory();
        let day = Local::now().date_naive();
        let start = local_midnight(day).unwrap();
        store.db.execute_batch("INSERT INTO tasks(id,title,estimate) VALUES(1,'Read 日本語 हिंदी',2),(2,'Read 日本語 हिंदी',1),(3,'Unused',1);").unwrap();
        // Multiple intervals in one completed session must credit completion only once.
        for (session, task, offset, active) in [
            (1, Some(1), 0, 120_000),
            (1, Some(1), 120_000, 60_000),
            (2, Some(2), 180_000, 90_000),
            (3, None, 270_000, 30_000),
            (4, Some(1), -120_000, 60_000), // Outside this period.
        ] {
            store.db.execute("INSERT INTO intervals(session_id,task_id,start_ms,end_ms,active_ms) VALUES(?1,?2,?3,?4,?5)", params![session, task, start + offset, start + offset + active, active]).unwrap();
        }
        store
            .db
            .execute(
                "INSERT INTO sessions(id,task_id,completed,ended_ms) VALUES(1,1,1,?1),(2,2,0,?1)",
                [start + 300_000],
            )
            .unwrap();
        store.execute(Operation::DeleteTask(1)).unwrap();
        let report = store.report_between(day, day.succ_opt().unwrap()).unwrap();
        assert_eq!(report.active_ms, 300_000);
        assert_eq!(report.partial_ms, 120_000);
        assert_eq!(report.completed, 1);
        assert_eq!(report.tasks.len(), 3);
        let first = &report.tasks[0];
        assert_eq!(first.task_id, Some(1));
        assert_eq!(first.title, "Read 日本語 हिंदी");
        assert!(first.deleted);
        assert_eq!((first.active_ms, first.completed), (180_000, 1));
        assert_eq!(report.tasks[1].task_id, Some(2));
        assert!(!report.tasks[1].deleted);
        assert_eq!(report.tasks[2].task_id, None);
        assert_eq!(report.tasks[2].title, "Without a task");
        assert!(!report.tasks[2].deleted);
        assert_eq!(
            report.tasks.iter().map(|t| t.active_ms).sum::<u64>(),
            report.active_ms
        );
        assert_eq!(
            report.tasks.iter().map(|t| t.completed).sum::<u32>(),
            report.completed
        );
        assert!(
            store
                .report_between(
                    day.succ_opt().unwrap(),
                    day.checked_add_days(Days::new(2)).unwrap()
                )
                .unwrap()
                .tasks
                .is_empty()
        );
    }
    #[test]
    fn task_reports_share_daily_clipping_and_completion_boundaries() {
        let store = memory();
        let day = Local::now().date_naive();
        let start = local_midnight(day).unwrap();
        let end = local_midnight(day.succ_opt().unwrap()).unwrap();
        // Different wall and active durations exercise proportional clipping and rounding.
        store.db.execute("INSERT INTO intervals(session_id,task_id,start_ms,end_ms,active_ms) VALUES(1,7,?1,?2,100001),(2,NULL,?3,?4,60000)", params![start - 60_000, start + 60_000, end - 30_000, end + 30_000]).unwrap();
        store.db.execute("INSERT INTO sessions(id,task_id,completed,ended_ms) VALUES(1,7,1,?1),(2,NULL,1,?2),(3,9,1,?3)", params![start + 60_000, end, start]).unwrap();
        for days in [1, 7, 31] {
            let report = store
                .report_between(day, day.checked_add_days(Days::new(days)).unwrap())
                .unwrap();
            assert_eq!(
                report.tasks.iter().map(|t| t.active_ms).sum::<u64>(),
                report.active_ms
            );
            assert_eq!(
                report.tasks.iter().map(|t| t.completed).sum::<u32>(),
                report.completed
            );
            let task = report.tasks.iter().find(|t| t.task_id == Some(7)).unwrap();
            assert_eq!(task.active_ms, 50_000);
            assert_eq!(task.title, "Deleted task #7");
            assert!(task.deleted);
            let no_intervals = report.tasks.iter().find(|t| t.task_id == Some(9)).unwrap();
            assert_eq!((no_intervals.active_ms, no_intervals.completed), (0, 1));
            let unassigned = report.tasks.iter().find(|t| t.task_id.is_none()).unwrap();
            assert_eq!(
                unassigned.active_ms,
                if days == 1 { 30_000 } else { 60_000 }
            );
            assert_eq!(unassigned.completed, u32::from(days > 1));
        }
    }
    #[test]
    fn week_and_month_follow_calendar_boundaries() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let (a, b) = date_range(date, Period::Week, 0).unwrap();
        assert_eq!(a.weekday(), chrono::Weekday::Mon);
        assert_eq!((b - a).num_days(), 7);
        let (a, b) = date_range(date, Period::Month, -1).unwrap();
        assert_eq!(a.to_string(), "2026-09-01");
        assert_eq!(b.to_string(), "2026-10-01");
    }
    #[test]
    fn completion_retry_does_not_double_task_credit() {
        let s = memory();
        for _ in 0..2 {
            s.db.execute("INSERT INTO sessions(id,task_id,completed,ended_ms) VALUES(1,1,1,0) ON CONFLICT(id) DO NOTHING",[]).unwrap();
        }
        assert_eq!(
            s.db.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
    }
}
