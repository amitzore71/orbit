#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use focus_core::{Clock, Command, Effect, Mode, Settings, Timer, WidgetPreferences};
use focus_storage::{Operation, Period, Report, Response, Task, Worker};
use platform_win::{Event, EventHandler, Platform, WindowsClock};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc, time::Duration};
slint::include_modules!();
const TASK_REQUEST: u64 = 1u64 << 62;
const CHECKPOINT_REQUEST: u64 = TASK_REQUEST + 1;
const CREATE_REQUEST: u64 = TASK_REQUEST + 2;
const SETTINGS_REQUEST: u64 = TASK_REQUEST + 3;
const DATA_REQUEST: u64 = TASK_REQUEST + 4;
const PAGE_TIMER: i32 = 0;
const PAGE_TASKS: i32 = 1;
const PAGE_REPORTS: i32 = 2;
#[cfg(feature = "diagnostics")]
mod diagnostics;
mod product;
#[cfg(test)]
mod regression_tests;
mod task_models;
thread_local! {static APP:RefCell<Option<Rc<RefCell<App>>>>=const{RefCell::new(None)};}
fn with_app(action: impl FnOnce(&mut App)) {
    let app = APP.with_borrow(Clone::clone);
    if let Some(app) = app {
        action(&mut app.borrow_mut());
    }
}
fn surface_visible(window: &slint::Window) -> bool {
    window.is_visible() && !window.is_minimized()
}
fn native_handler() -> EventHandler {
    Arc::new(|event| {
        let _ = slint::invoke_from_event_loop(move || with_app(|app| app.native_event(event)));
    })
}
fn window_handle(window: &slint::Window) -> Option<isize> {
    window
        .window_handle()
        .window_handle()
        .ok()
        .and_then(|h| match h.as_raw() {
            RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
            _ => None,
        })
}
enum Widget {
    Timer(TimerWidget),
    Tasks(TasksWidget),
    Mini(MiniWidget),
}
impl Widget {
    fn window(&self) -> &slint::Window {
        match self {
            Self::Timer(w) => w.window(),
            Self::Tasks(w) => w.window(),
            Self::Mini(w) => w.window(),
        }
    }
}
struct App {
    #[cfg(feature = "diagnostics")]
    diagnostic_ticks: u64,
    main: MainWindow,
    timer: Timer,
    storage: Worker,
    platform: Platform,
    tasks: Vec<Task>,
    task_models: task_models::TaskModels,
    preferences: [WidgetPreferences; 3],
    widgets: [Option<Widget>; 3],
    display_timer: slint::Timer,
    expiry_timer: slint::Timer,
    checkpoint_timer: slint::Timer,
    calendar_timer: slint::Timer,
    calendar_date: String,
    checkpoint_pending: bool,
    start_after_create: bool,
    create_pending: bool,
    detail_generation: u64,
    period: Period,
    report_offset: i32,
    report_generation: u64,
    today_generation: u64,
    report_in_flight: Option<u64>,
    today_in_flight: Option<u64>,
    tasks_in_flight: bool,
    tasks_dirty: bool,
    data_dir: PathBuf,
    quitting: bool,
    main_attached: bool,
    widget_attached: [bool; 3],
    main_requested_visible: bool,
}
fn main() -> std::process::ExitCode {
    if let Err(error) = run() {
        if !std::env::args().any(|a| a == "--register-notifications") {
            platform_win::message_box("Orbit could not start", &error.to_string());
        }
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|s| s == "--register-notifications") {
        platform_win::register_notifications()?;
        return Ok(());
    }
    let data_dir = args
        .iter()
        .position(|s| s == "--data-dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(platform_win::data_directory);
    let instance_name = if args.iter().any(|s| s == "--isolated") {
        format!("Local\\StillPomodoro.{}", std::process::id())
    } else {
        "Local\\StillPomodoro.Desktop.v1".into()
    };
    let Some(_instance) = platform_win::Instance::acquire(&instance_name)? else {
        return Ok(());
    };
    #[cfg(not(feature = "gpu"))]
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    #[cfg(feature = "gpu")]
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    let (storage, loaded) = Worker::open(platform_win::database_path(&data_dir))?;
    let main = MainWindow::new()?;
    let timer = if let Some(checkpoint) = loaded.checkpoint {
        Timer::restore(loaded.settings.clone(), checkpoint)
    } else {
        Timer::new(loaded.settings.clone())
    };
    let platform = Platform::start(native_handler())?;
    let mut warnings = platform.hotkeys(&loaded.settings);
    // An existing opt-in follows this executable after the Orbit rename.
    if loaded.settings.autostart
        && let Err(error) = platform_win::set_autostart(true)
    {
        warnings.push(error);
    }
    let mut preferences = loaded.widgets;
    if args.iter().any(|s| s == "--all-widgets") {
        for p in &mut preferences {
            p.enabled = true;
        }
    }
    let app = Rc::new(RefCell::new(App {
        #[cfg(feature = "diagnostics")]
        diagnostic_ticks: 0,
        main,
        timer,
        storage,
        platform,
        tasks: loaded.tasks,
        task_models: task_models::TaskModels::default(),
        preferences,
        widgets: Default::default(),
        display_timer: Default::default(),
        expiry_timer: Default::default(),
        checkpoint_timer: Default::default(),
        calendar_timer: Default::default(),
        calendar_date: focus_storage::local_date(),
        checkpoint_pending: false,
        start_after_create: false,
        create_pending: false,
        detail_generation: TASK_REQUEST + 1024,
        period: Period::Week,
        report_offset: 0,
        report_generation: 0,
        today_generation: 0,
        report_in_flight: None,
        today_in_flight: None,
        tasks_in_flight: false,
        tasks_dirty: false,
        data_dir,
        quitting: false,
        main_attached: false,
        widget_attached: [false; 3],
        main_requested_visible: !args.iter().any(|s| s == "--tray" || s == "--minimized"),
    }));
    APP.with_borrow_mut(|slot| *slot = Some(app.clone()));
    {
        let mut a = app.borrow_mut();
        a.wire_main();
        a.wire_product();
        if let Some(path) = &loaded.last_backup {
            a.main
                .set_data_status(format!("Last backup: {path}").into());
        }
        a.load_settings();
        let selected = a.timer.checkpoint(WindowsClock.now()).selected_task;
        if selected.is_some_and(|id| !a.tasks.iter().any(|t| t.id == id && !t.done && !t.deleted)) {
            a.command(Command::SelectTask(None));
        }
        a.refresh_tasks();
        a.refresh_widget_rows();
        if loaded.recovered {
            a.message("An interrupted session was restored paused. Recovery is approximate; unsaved running time was not added to reports.");
        }
        if !warnings.is_empty() {
            a.message(&warnings.join(" "));
        }
        if !args.iter().any(|s| s == "--tray") {
            a.main.show()?;
            if args.iter().any(|s| s == "--minimized") {
                a.main.window().set_minimized(true);
            }
        }
        for i in 0..3 {
            if a.preferences[i].enabled {
                a.create_widget(i);
            }
        }
        a.request_today();
        a.render();
        a.reschedule();
    }
    // Raw native handles become available after the backend creates its windows.
    slint::Timer::single_shot(Duration::from_millis(80), || {
        with_app(|a| a.attach_windows())
    });
    if args.iter().any(|s| s == "--running") {
        with_app(|a| a.command(Command::Start));
    }
    if let Some(seconds) = args
        .iter()
        .position(|s| s == "--quit-after")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u64>().ok())
    {
        slint::Timer::single_shot(Duration::from_secs(seconds), || with_app(|a| a.quit()));
    }
    #[cfg(feature = "diagnostics")]
    if let Some(cycles) = args
        .iter()
        .position(|s| s == "--stress-cycles")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u32>().ok())
    {
        diagnostics::start(cycles, app.borrow().data_dir.join("stress-progress.json"));
    }
    #[cfg(feature = "diagnostics")]
    if args.iter().any(|argument| argument == "--ui-audit") {
        diagnostics::audit(app.borrow().data_dir.join("screenshots"));
    }
    #[cfg(feature = "diagnostics")]
    if args
        .iter()
        .any(|argument| argument == "--performance-audit")
    {
        diagnostics::performance(app.borrow().data_dir.join("performance.json"));
    }
    slint::run_event_loop_until_quit()?;
    APP.with_borrow_mut(|slot| *slot = None);
    Ok(())
}
impl App {
    fn message(&self, message: &str) {
        self.main.set_message(message.into());
    }
    fn wire_main(&self) {
        self.main
            .on_control(|name| with_app(|a| a.control(name.as_str())));
        self.main.on_select_mode(|index| {
            with_app(|a| a.command(Command::SelectMode(Mode::from_index(index))))
        });
        self.main.on_task_action(|id, action| {
            with_app(|a| a.task_action(i64::from(id), action.as_str()))
        });
        self.main.on_save_task(|| with_app(|a| a.save_task()));
        self.main
            .on_save_settings(|| with_app(|a| a.save_settings()));
        self.main.on_preview_sound(|| {
            with_app(|a| {
                platform_win::play_alarm(
                    a.main.get_sound() as u32,
                    a.main.get_volume() as u32,
                    native_handler(),
                )
            })
        });
        self.main.on_report_change(|period, step| {
            with_app(|a| {
                a.main.set_range_start("".into());
                a.main.set_range_end("".into());
                let period = match period {
                    0 => Period::Day,
                    2 => Period::Month,
                    _ => Period::Week,
                };
                if std::mem::discriminant(&period) != std::mem::discriminant(&a.period) {
                    a.report_offset = 0;
                }
                a.period = period;
                if step == 2 {
                    a.report_offset = 0;
                } else {
                    a.report_offset = (a.report_offset + step).clamp(-1200, 1200);
                }
                a.command(Command::Checkpoint);
            })
        });
        self.main.on_widget_change(|index, enabled, layer, click| {
            with_app(|a| {
                let index = index as usize;
                if index >= 3 {
                    return;
                }
                a.preferences[index].enabled = enabled;
                a.preferences[index].layer = layer.clamp(0, 2);
                a.preferences[index].click_through = click;
                if enabled {
                    a.create_widget(index);
                } else {
                    a.destroy_widget(index);
                }
                a.apply_widget(index);
                a.save_widget(index);
                a.refresh_widget_rows();
                a.render();
                a.reschedule();
            })
        });
        self.main
            .on_widget_reset(|index| with_app(|a| a.reset_widget(index as usize)));
        self.main.on_register_notifications(|| {
            let result = platform_win::register_notifications();
            with_app(|a| match result {
                Ok(()) => {
                    a.message(
                        "Notifications are registered for this executable. A test alert was sent.",
                    );
                    a.platform
                        .notify("Orbit notifications", "This is a test alert.");
                }
                Err(e) => a.message(&e),
            })
        });
        self.main.on_open_data_folder(|| {
            let mut path = None;
            with_app(|a| path = Some(a.data_dir.clone()));
            if let Some(path) = path {
                platform_win::open_folder(&path);
            }
        });
        self.main.on_navigate(|page| {
            with_app(|a| {
                a.cancel_capture();
                a.refresh_calendar();
                if page == PAGE_TASKS {
                    a.task_models
                        .filter(a.main.get_task_view(), a.main.get_task_search().as_str());
                }
                // Returning to the timer must refresh its value and restart display
                // updates even after a page without a timer suspended those updates.
                a.command(if page == PAGE_REPORTS {
                    Command::Checkpoint
                } else {
                    Command::Tick
                });
            })
        });
        self.main.window().on_close_requested(|| {
            let _ = slint::invoke_from_event_loop(|| with_app(|a| a.control("hide")));
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    fn control(&mut self, name: &str) {
        match name {
            "retry" => {
                self.main.set_save_status("Retrying saved changes…".into());
                self.submit(Operation::Flush, 0);
            }
            "discard-settings" => {
                self.cancel_capture();
                self.load_settings();
            }
            "quit-discard" => {
                self.load_settings();
                self.quit();
            }
            "backup" => self.begin_file_dialog(0),
            "restore" => self.begin_file_dialog(1),
            "export" => self.begin_file_dialog(2),
            "restore-confirm" => {
                let path = PathBuf::from(self.main.get_restore_path().as_str());
                self.main.set_restore_path("".into());
                self.main
                    .set_data_busy(self.submit(Operation::Restore(path), DATA_REQUEST));
            }
            "toggle" => self.command(Command::Toggle),
            "skip" => self.command(Command::Skip),
            "reset" => self.command(Command::Reset),
            "show" => self.show(),
            "hide" => {
                self.main_requested_visible = false;
                self.command(Command::Checkpoint);
                if let Err(e) = self.main.hide() {
                    self.message(&e.to_string());
                }
                self.reschedule();
            }
            "minimize" => {
                self.main.window().set_minimized(true);
                self.reschedule();
            }
            "quit" => self.quit(),
            _ => {}
        }
    }
    fn command(&mut self, command: Command) {
        if self.main.get_data_busy()
            && !matches!(
                command,
                Command::Tick
                    | Command::Checkpoint
                    | Command::Pause
                    | Command::Suspend
                    | Command::Resume
            )
        {
            self.message("Finish the file operation before changing the timer.");
            return;
        }
        #[cfg(feature = "diagnostics")]
        if matches!(command, Command::Tick) {
            self.diagnostic_ticks += 1;
        }
        if self.quitting {
            return;
        }
        if self.storage.is_blocked() && matches!(command, Command::Suspend | Command::Resume) {
            let now = WindowsClock.now();
            if self.timer.snapshot(now).running {
                let effects = self.timer.dispatch(Command::Pause, now);
                if let Err(error) = self
                    .storage
                    .preserve_pause(effects, self.timer.checkpoint(now))
                {
                    self.message(&error);
                }
            }
            self.timer.dispatch(command, now);
            self.render();
            self.reschedule();
            return;
        }
        if self.storage.is_blocked() && !matches!(command, Command::Tick) {
            self.submit(Operation::Flush, 0);
            self.message("Saving was interrupted. Retrying saved changes; the timer stays paused. Try the control again after saving recovers.");
            return;
        }
        if matches!(command, Command::Checkpoint) && self.checkpoint_pending {
            return;
        }
        let now = WindowsClock.now();
        let previous = self.timer.clone();
        let effects = self.timer.dispatch(command, now);
        let tasks_changed = effects
            .iter()
            .any(|effect| matches!(effect, Effect::SessionEnded { .. }));
        let changed = !matches!(command, Command::Tick) || !effects.is_empty();
        let completed: Vec<Mode> = effects
            .iter()
            .filter_map(|e| {
                if let Effect::Completed(m) = e {
                    Some(*m)
                } else {
                    None
                }
            })
            .collect();
        if changed {
            if !self.submit(
                Operation::Transition {
                    effects,
                    checkpoint: self.timer.checkpoint(now),
                },
                if matches!(command, Command::Checkpoint) {
                    CHECKPOINT_REQUEST
                } else {
                    0
                },
            ) {
                self.timer = previous;
                self.reschedule();
                return;
            }
            if matches!(command, Command::Checkpoint) {
                self.checkpoint_pending = true;
            }
            if tasks_changed {
                self.request_tasks();
            }
            self.request_today();
            if self.main.get_page() == PAGE_REPORTS {
                self.request_report();
            }
            let snapshot = self.timer.snapshot(now);
            self.platform.tooltip(&format!(
                "Orbit · {} · {}",
                snapshot.mode.label(),
                if snapshot.running {
                    "Running"
                } else {
                    "Paused"
                }
            ));
        }
        for mode in completed {
            let title = if mode == Mode::Focus {
                "Focus session complete"
            } else {
                "Break complete"
            };
            platform_win::play_alarm(
                self.timer.settings.sound,
                self.timer.settings.volume,
                native_handler(),
            );
            if self.timer.settings.notifications {
                self.platform.notify(
                    title,
                    &format!("Next session: {}.", self.timer.snapshot(now).mode.label()),
                );
            }
        }
        self.render();
        self.reschedule();
    }
    fn render(&self) {
        self.sync_layout();
        let main_visible = surface_visible(self.main.window());
        let main_timer = main_visible && self.main.get_page() == PAGE_TIMER;
        let ring_widget =
            self.widgets.iter().flatten().any(|widget| {
                matches!(widget, Widget::Timer(_)) && surface_visible(widget.window())
            });
        let mini_widget =
            self.widgets.iter().flatten().any(|widget| {
                matches!(widget, Widget::Mini(_)) && surface_visible(widget.window())
            });
        let snapshot = self.timer.snapshot(WindowsClock.now());
        self.main.set_storage_blocked(self.storage.is_blocked());
        self.main.set_running(snapshot.running);
        self.main.set_started(snapshot.started);
        self.main
            .set_can_start_task(!snapshot.started && snapshot.mode == Mode::Focus);
        if main_visible {
            // Other pages also use the current mode's accent after a session ends.
            self.main.set_mode(snapshot.mode.index());
        }
        for widget in self.widgets.iter().flatten() {
            match widget {
                Widget::Timer(w) => {
                    w.set_storage_blocked(self.storage.is_blocked());
                    w.set_started(snapshot.started);
                }
                Widget::Mini(w) => {
                    w.set_storage_blocked(self.storage.is_blocked());
                    w.set_started(snapshot.started);
                }
                Widget::Tasks(w) => {
                    w.set_storage_blocked(self.storage.is_blocked());
                    w.set_started(snapshot.started);
                }
            }
        }
        if !main_timer && !ring_widget && !mini_widget {
            return;
        }
        let label: SharedString = snapshot.time_label().into();
        let mode_label: SharedString = snapshot.mode.label().into();
        let path: SharedString = if main_timer || ring_widget {
            ring_path(1.0 - snapshot.progress()).into()
        } else {
            SharedString::default()
        };
        if main_timer {
            self.main.set_time_text(label.clone());
            self.main.set_ring_path(path.clone());
            self.main.set_mode_label(mode_label.clone());
            self.main.set_running(snapshot.running);
            self.main.set_cycle(snapshot.cycle as i32);
            self.main.set_state_label(
                if snapshot.running {
                    "Running"
                } else if snapshot.started {
                    "Paused"
                } else {
                    "Ready"
                }
                .into(),
            );
            let task = snapshot
                .task
                .and_then(|id| self.tasks.iter().find(|t| t.id == id))
                .map(|t| {
                    if t.deleted {
                        format!("{} · Deleted task", t.title)
                    } else {
                        t.title.clone()
                    }
                })
                .unwrap_or_else(|| "Focus without a task".into());
            self.main.set_active_task(task.into());
            let checkpoint = self.timer.checkpoint(WindowsClock.now());
            let next = if snapshot.started && checkpoint.selected_task != checkpoint.bound_task {
                checkpoint
                    .selected_task
                    .and_then(|id| self.tasks.iter().find(|t| t.id == id))
                    .map(|t| format!("Next focus: {}", t.title))
                    .unwrap_or_else(|| "Next focus: without a task".into())
            } else {
                String::new()
            };
            self.main.set_next_task(next.into());
            let completed = snapshot.cycle % self.timer.settings.long_interval.max(1);
            self.main.set_cycle_label(
                format!(
                    "{} of {} focus sessions before a long break",
                    if completed == 0 && snapshot.cycle > 0 && snapshot.mode == Mode::LongBreak {
                        self.timer.settings.long_interval
                    } else {
                        completed
                    },
                    self.timer.settings.long_interval
                )
                .into(),
            );
        }
        let tokens = self.main.global::<Tokens>();
        let accent = match snapshot.mode {
            Mode::Focus => tokens.get_coral(),
            Mode::ShortBreak => tokens.get_teal(),
            Mode::LongBreak => tokens.get_indigo(),
        };
        for widget in self.widgets.iter().flatten() {
            if !surface_visible(widget.window()) {
                continue;
            }
            match widget {
                Widget::Timer(w) => {
                    w.set_time_text(label.clone());
                    w.set_ring_path(path.clone());
                    w.set_mode_label(mode_label.clone());
                    w.set_running(snapshot.running);
                    w.set_accent(accent);
                }
                Widget::Mini(w) => {
                    w.set_time_text(label.clone());
                    w.set_mode_label(mode_label.clone());
                    w.set_running(snapshot.running);
                }
                Widget::Tasks(_) => {}
            }
        }
    }
    fn reschedule(&self) {
        self.display_timer.stop();
        self.expiry_timer.stop();
        let snapshot = self.timer.snapshot(WindowsClock.now());
        if !snapshot.running || self.quitting || self.storage.is_blocked() {
            self.checkpoint_timer.stop();
            return;
        }
        if !self.checkpoint_timer.running() {
            self.checkpoint_timer.start(
                slint::TimerMode::Repeated,
                Duration::from_secs(60),
                || with_app(|a| a.command(Command::Checkpoint)),
            );
        }
        let visible = (surface_visible(self.main.window()) && self.main.get_page() == PAGE_TIMER)
            || self
                .widgets
                .iter()
                .flatten()
                .any(|w| !matches!(w, Widget::Tasks(_)) && surface_visible(w.window()));
        if visible {
            let until_next = ((snapshot.remaining_ms.saturating_sub(1)) % 1000 + 1).max(1);
            self.display_timer.start(
                slint::TimerMode::SingleShot,
                Duration::from_millis(until_next),
                || with_app(|a| a.command(Command::Tick)),
            );
        } else {
            self.expiry_timer.start(
                slint::TimerMode::SingleShot,
                Duration::from_millis(snapshot.remaining_ms.max(1)),
                || with_app(|a| a.command(Command::Tick)),
            );
        }
    }
    fn submit(&self, operation: Operation, generation: u64) -> bool {
        if self.quitting {
            return false;
        }
        let result = self.storage.submit(operation, move |result| {
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|a| a.storage_response(generation, result))
            });
        });
        if let Err(e) = result {
            self.message(&format!("Changes could not be saved: {e}"));
            return false;
        }
        true
    }
    fn storage_response(&mut self, generation: u64, result: focus_storage::Result<Response>) {
        if generation == CHECKPOINT_REQUEST {
            self.checkpoint_pending = false;
        }
        self.main.set_storage_blocked(self.storage.is_blocked());
        if generation == TASK_REQUEST {
            self.tasks_in_flight = false;
            if self.tasks_dirty {
                self.tasks_dirty = false;
                self.request_tasks();
                if result.is_ok() {
                    return;
                }
            }
        }
        if self.report_in_flight == Some(generation) {
            self.report_in_flight = None;
        }
        if self.today_in_flight == Some(generation) {
            self.today_in_flight = None;
        }
        if generation > 0 && generation < TASK_REQUEST && generation != self.report_generation {
            if self.report_in_flight.is_none() {
                self.send_report();
            }
            return;
        }
        if (TASK_REQUEST + 1024..(1u64 << 63)).contains(&generation)
            && generation != self.detail_generation
        {
            return;
        }
        match result {
            Err(e) => {
                if generation > 0 && generation < TASK_REQUEST && !self.storage.is_blocked() {
                    self.main.set_range_error(e.into());
                    return;
                }
                if generation == self.detail_generation {
                    self.main.set_detail_rows(ModelRc::new(VecModel::from(vec![
                        SharedString::from(format!("Could not load details: {e}")),
                    ])));
                }

                if generation == SETTINGS_REQUEST || self.storage.is_blocked() {
                    self.main.set_settings_saving(false);
                }
                if generation == DATA_REQUEST || self.storage.is_blocked() {
                    self.main.set_data_busy(false);
                }
                if generation == CREATE_REQUEST || self.storage.is_blocked() {
                    self.main.set_task_saving(false);
                    self.start_after_create = false;
                }
                self.main
                    .set_save_status("Could not save — timer paused".into());
                if self.storage.is_blocked() && self.timer.snapshot(WindowsClock.now()).running {
                    let now = WindowsClock.now();
                    let effects = self.timer.dispatch(Command::Pause, now);
                    if let Err(error) = self
                        .storage
                        .preserve_pause(effects, self.timer.checkpoint(now))
                    {
                        self.message(&error);
                    }
                    self.render();
                    self.reschedule();
                }
                self.message(&if self.storage.is_blocked() {
                    format!(
                        "Storage: {e}. Activity is paused; retry a timer control to retry saving."
                    )
                } else {
                    format!("Storage: {e}")
                });
            }
            Ok(Response::Tasks(tasks)) => {
                self.tasks = tasks;
                let selected = self.timer.checkpoint(WindowsClock.now()).selected_task;
                if selected.is_some_and(|id| {
                    !self
                        .tasks
                        .iter()
                        .any(|t| t.id == id && !t.done && !t.deleted)
                }) && !self.storage.is_blocked()
                {
                    self.command(Command::SelectTask(None));
                }
                self.refresh_tasks();
                self.render();
            }
            Ok(Response::TasksChanged) => self.request_tasks(),
            Ok(Response::TaskCreated(task)) => {
                self.create_pending = false;
                let id = task.id;
                self.tasks.push(task);
                self.main.set_task_saving(false);
                self.main.set_task_title("".into());
                self.main.set_task_estimate(1);
                self.refresh_tasks();
                if std::mem::take(&mut self.start_after_create) {
                    self.command(Command::StartTask(id));
                }
                self.request_tasks();
            }
            Ok(Response::TaskDeleted(id)) => {
                self.main.set_undo_id(id as i32);
                slint::Timer::single_shot(Duration::from_secs(10), move || {
                    with_app(|a| {
                        if a.main.get_undo_id() == id as i32 {
                            a.main.set_undo_id(-1);
                        }
                    })
                });
                self.request_tasks();
            }
            Ok(Response::BackedUp(path)) => {
                self.main.set_data_busy(false);
                self.main.set_data_status(
                    format!("Backup saved: {}. The timer is paused.", path.display()).into(),
                );
            }
            Ok(Response::SessionDetails(rows)) => {
                if generation == self.detail_generation {
                    self.main.set_detail_rows(ModelRc::new(VecModel::from(
                        rows.into_iter().map(SharedString::from).collect::<Vec<_>>(),
                    )));
                }
            }
            Ok(Response::Exported(path)) => {
                self.main.set_data_busy(false);
                self.message(&format!("Report exported: {}", path.display()));
            }
            Ok(Response::Restored(loaded)) => self.apply_restore(*loaded),
            Ok(Response::SettingsSaved(settings)) => {
                self.main.set_settings_saving(false);
                self.apply_saved_settings(settings);
            }
            Ok(Response::Report(report)) => {
                if generation == self.report_generation {
                    self.show_report(report);
                }
            }
            Ok(Response::Today(summary)) => {
                if generation == self.today_generation {
                    self.main.set_today_label(
                        format!(
                            "{} today · {} completed",
                            duration_label(summary.active_ms),
                            summary.completed
                        )
                        .into(),
                    );
                    for widget in self.widgets.iter().flatten() {
                        if let Widget::Tasks(w) = widget {
                            w.set_today_label(
                                format!(
                                    "{} focus · {} completed",
                                    duration_label(summary.active_ms),
                                    summary.completed
                                )
                                .into(),
                            );
                        }
                    }
                }
            }
            Ok(Response::Saved) => {
                self.main.set_save_status("Saved".into());
            }
            Ok(Response::Flushed(settings)) => {
                if std::mem::take(&mut self.create_pending) {
                    self.main.set_task_title("".into());
                    self.main.set_task_estimate(1);
                    self.start_after_create = false;
                }
                self.main.set_settings_saving(false);
                self.main.set_task_saving(false);
                self.main
                    .set_save_status("Changes saved. Ready to resume.".into());
                self.apply_saved_settings(settings);
                self.request_tasks();
                self.request_today();
                if self.main.get_page() == PAGE_REPORTS {
                    self.request_report();
                }
                self.message("Changes saved. The timer is paused and ready to resume.");
            }
        }
        if self.report_in_flight.is_none()
            && generation != 0
            && generation & (1u64 << 63) == 0
            && generation < TASK_REQUEST
            && generation != self.report_generation
        {
            self.send_report();
        }
        if self.today_in_flight.is_none()
            && generation & (1u64 << 63) != 0
            && generation != self.today_generation
        {
            self.send_today();
        }
        if self.tasks_dirty && !self.tasks_in_flight && !self.quitting {
            self.request_tasks();
        }
    }
    fn request_tasks(&mut self) {
        if self.tasks_in_flight {
            self.tasks_dirty = true;
        } else {
            self.tasks_dirty = false;
            if self.submit(Operation::Tasks, TASK_REQUEST) {
                self.tasks_in_flight = true;
            } else {
                self.tasks_dirty = true;
            }
        }
    }
    fn refresh_tasks(&self) {
        let selected = self.timer.checkpoint(WindowsClock.now()).selected_task;
        self.task_models.rebuild(&self.tasks, selected);
        self.task_models.filter(
            self.main.get_task_view(),
            self.main.get_task_search().as_str(),
        );
        self.main
            .set_tasks(self.task_models.filtered.clone().into());
        self.main
            .set_picker_tasks(self.task_models.active.clone().into());
        for widget in self.widgets.iter().flatten() {
            if let Widget::Tasks(w) = widget {
                w.set_tasks(self.task_models.today.clone().into());
            }
        }
    }
    fn refresh_task_selection(&self) {
        self.task_models
            .select(self.timer.checkpoint(WindowsClock.now()).selected_task);
    }
    fn save_task(&mut self) {
        if self.main.get_task_saving() {
            return;
        }
        let title = self.main.get_task_title().to_string();
        let estimate = self.main.get_task_estimate().clamp(1, 99) as u32;
        if title.trim().is_empty() || title.chars().count() > 240 {
            self.start_after_create = false;
            self.message("Write a task title with 1–240 characters.");
            return;
        }
        let id = self.main.get_edit_id();
        let operation = if id < 0 {
            Operation::CreateTask {
                title,
                estimate,
                today: self.main.get_task_view() == 0 || self.main.get_page() == PAGE_TIMER,
            }
        } else {
            Operation::EditTask {
                id: i64::from(id),
                title,
                estimate,
            }
        };
        if !self.submit(operation, if id < 0 { CREATE_REQUEST } else { 0 }) {
            self.start_after_create = false;
            return;
        }
        if id < 0 {
            self.create_pending = true;
            self.main.set_task_saving(true);
        } else {
            self.main.set_task_title("".into());
            self.main.set_task_estimate(1);
            self.main.set_edit_id(-1);
        }
    }
    fn task_action(&mut self, id: i64, action: &str) {
        if action == "start" {
            if self
                .tasks
                .iter()
                .any(|t| t.id == id && !t.done && !t.deleted)
            {
                self.command(Command::StartTask(id));
                self.main.set_page(PAGE_TIMER);
                self.render();
                self.reschedule();
            }
            return;
        }
        if action == "select" {
            self.command(Command::SelectTask(if id < 0 { None } else { Some(id) }));
            self.refresh_task_selection();
            return;
        }
        let Some(task) = self.tasks.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        match action {
            "edit" => {
                self.main.set_edit_id(id as i32);
                self.main.set_task_title(task.title.into());
                self.main.set_task_estimate(task.estimate as i32);
            }
            "done" => {
                self.submit(Operation::ToggleTaskDone(id), 0);
            }
            "delete" => {
                self.submit(Operation::DeleteTask(id), 0);
            }
            "restore" => {
                self.submit(Operation::RestoreTask(id), 0);
                self.main.set_undo_id(-1);
            }
            "plan" => {
                self.submit(Operation::PlanTask { id, today: true }, 0);
            }
            "unplan" => {
                self.submit(Operation::PlanTask { id, today: false }, 0);
            }
            "up" | "down" => {
                self.submit(
                    Operation::MoveTask {
                        id,
                        direction: if action == "up" { -1 } else { 1 },
                    },
                    0,
                );
            }
            "details" => {
                self.main.set_detail_title(task.title.into());
                self.main
                    .set_detail_rows(ModelRc::new(VecModel::from(vec![SharedString::from(
                        "Task history is preserved across completion and deletion.",
                    )])));
            }
            _ => {}
        }
    }
    fn request_report(&mut self) {
        self.report_generation = (self.report_generation + 1) & (TASK_REQUEST - 1);
        if self.report_in_flight.is_none() {
            self.send_report();
        }
    }
    fn send_report(&mut self) {
        let operation =
            if self.main.get_range_start().is_empty() && self.main.get_range_end().is_empty() {
                Operation::Report {
                    period: self.period,
                    offset: self.report_offset,
                }
            } else {
                Operation::ReportRange {
                    start: self.main.get_range_start().to_string(),
                    end: self.main.get_range_end().to_string(),
                }
            };
        if self.submit(operation, self.report_generation) {
            self.report_in_flight = Some(self.report_generation);
        }
    }
    fn request_today(&mut self) {
        self.today_generation = ((self.today_generation + 1) & (TASK_REQUEST - 1)) | (1u64 << 63);
        if self.today_in_flight.is_none() {
            self.send_today();
        }
    }
    fn send_today(&mut self) {
        if self.submit(Operation::Today, self.today_generation) {
            self.today_in_flight = Some(self.today_generation);
        }
    }
    fn show_report(&self, mut report: Report) {
        self.main.set_range_error("".into());
        let chunk_size = if report.days.len() > 31 {
            report.days.len().div_ceil(14)
        } else {
            1
        };
        self.main.set_chart_note(if chunk_size > 1 {
            format!("Each bar groups up to {chunk_size} days. Export includes every day.").into()
        } else {
            "".into()
        });
        if chunk_size > 1 {
            report.days = report
                .days
                .chunks(chunk_size)
                .map(|chunk| focus_storage::DayReport {
                    label: chunk[0].label.clone(),
                    active_ms: chunk.iter().map(|d| d.active_ms).sum(),
                    completed: chunk.iter().map(|d| d.completed).sum(),
                })
                .collect();
        }

        self.main.set_report_title(report.title.into());
        self.main
            .set_report_time(duration_label(report.active_ms).into());
        self.main
            .set_report_partial(duration_label(report.partial_ms).into());
        self.main.set_report_completed(report.completed as i32);
        self.main.set_report_tasks(ModelRc::new(VecModel::from(
            report
                .tasks
                .into_iter()
                .map(|task| {
                    let fraction = if report.active_ms == 0 {
                        0.0
                    } else {
                        task.active_ms as f32 / report.active_ms as f32
                    };
                    ReportTask {
                        id: task.task_id.map_or(-1, |id| id as i32),
                        title: task.title.into(),
                        time: duration_label(task.active_ms).into(),
                        share: format!("{:.1}%", fraction * 100.0).into(),
                        detail: format!(
                            "{} completed session{}{}",
                            task.completed,
                            if task.completed == 1 { "" } else { "s" },
                            if task.deleted { " · Deleted task" } else { "" }
                        )
                        .into(),
                        fraction,
                    }
                })
                .collect::<Vec<_>>(),
        )));
        let task_model = self.main.get_report_tasks();
        self.main.set_report_preview(ModelRc::new(VecModel::from(
            (0..task_model.row_count().min(3))
                .filter_map(|index| task_model.row_data(index))
                .collect::<Vec<_>>(),
        )));
        let maximum = report
            .days
            .iter()
            .map(|d| d.active_ms)
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        self.main.set_report_days(ModelRc::new(VecModel::from(
            report
                .days
                .into_iter()
                .map(|d| ReportDay {
                    label: d.label.into(),
                    minutes: duration_label(d.active_ms).into(),
                    fraction: d.active_ms as f32 / maximum,
                })
                .collect::<Vec<_>>(),
        )));
    }
    fn load_settings(&self) {
        let s = &self.timer.settings;
        self.main.set_focus_minutes(s.focus_minutes as i32);
        self.main.set_short_minutes(s.short_minutes as i32);
        self.main.set_long_minutes(s.long_minutes as i32);
        self.main.set_long_interval(s.long_interval as i32);
        self.main.set_auto_breaks(s.auto_breaks);
        self.main.set_auto_focus(s.auto_focus);
        self.main.set_volume(s.volume as i32);
        self.main.set_sound(s.sound as i32);
        self.main.set_notifications(s.notifications);
        self.main.set_reduced_motion(s.reduced_motion);
        self.main.set_mica(s.mica);
        self.main.set_autostart(s.autostart);
        self.main.set_toggle_hotkey(s.hotkey_toggle.clone().into());
        self.main.set_show_hotkey(s.hotkey_show.clone().into());
        self.main.set_skip_hotkey(s.hotkey_skip.clone().into());
        self.main.set_appearance(s.appearance as i32);
        self.main.set_saved_settings(product::draft(s));
        self.apply_appearance();
    }
    fn save_settings(&mut self) {
        self.cancel_capture();
        let s = Settings {
            focus_minutes: self.main.get_focus_minutes() as u32,
            short_minutes: self.main.get_short_minutes() as u32,
            long_minutes: self.main.get_long_minutes() as u32,
            long_interval: self.main.get_long_interval() as u32,
            auto_breaks: self.main.get_auto_breaks(),
            auto_focus: self.main.get_auto_focus(),
            volume: self.main.get_volume() as u32,
            sound: self.main.get_sound() as u32,
            notifications: self.main.get_notifications(),
            reduced_motion: self.main.get_reduced_motion(),
            mica: self.main.get_mica(),
            autostart: self.main.get_autostart(),
            hotkey_toggle: self.main.get_toggle_hotkey().to_string(),
            hotkey_show: self.main.get_show_hotkey().to_string(),
            hotkey_skip: self.main.get_skip_hotkey().to_string(),
            appearance: self.main.get_appearance() as u32,
        };
        if let Err(e) = s
            .validate()
            .and_then(|_| platform_win::validate_hotkeys(&s))
        {
            self.main.set_settings_error(e.clone().into());
            self.message(&e);
            return;
        }
        self.main.set_settings_error("".into());
        if self.submit(Operation::Settings(s), SETTINGS_REQUEST) {
            self.main.set_settings_saving(true);
            self.message("Saving settings…");
        }
    }
    fn apply_saved_settings(&mut self, settings: Settings) {
        let mut warnings = Vec::new();
        if settings.autostart != self.timer.settings.autostart
            && let Err(error) = platform_win::set_autostart(settings.autostart)
        {
            warnings.push(format!(
                "Startup registration could not be changed: {error}"
            ));
        }
        if settings != self.timer.settings {
            warnings.extend(self.platform.hotkeys(&settings));
            self.timer.settings = settings;
            let now = WindowsClock.now();
            let snapshot = self.timer.snapshot(now);
            if !snapshot.started {
                self.timer.dispatch(Command::SelectMode(snapshot.mode), now);
            }
        }
        self.main
            .set_saved_settings(product::draft(&self.timer.settings));
        self.apply_appearance();
        self.main.set_settings_error(warnings.join(" ").into());
        let message = if warnings.is_empty() {
            "Settings saved. New durations apply to the next session.".to_string()
        } else {
            warnings.join(" ")
        };
        self.message(&message);
    }
    fn create_widget(&mut self, index: usize) {
        if index >= 3 || self.widgets[index].is_some() {
            return;
        }
        let result: Result<Widget, slint::PlatformError> = match index {
            0 => TimerWidget::new().map(Widget::Timer),
            1 => TasksWidget::new().map(Widget::Tasks),
            _ => MiniWidget::new().map(Widget::Mini),
        };
        let widget = match result {
            Ok(w) => w,
            Err(e) => {
                self.preferences[index].enabled = false;
                self.message(&e.to_string());
                return;
            }
        };
        let control = move |name: SharedString| {
            with_app(|a| {
                if name.as_str() == "close" {
                    a.preferences[index].enabled = false;
                    a.destroy_widget(index);
                    a.save_widget(index);
                    a.refresh_widget_rows();
                    a.reschedule();
                } else {
                    a.control(name.as_str());
                }
            })
        };
        match &widget {
            Widget::Timer(w) => {
                w.on_control(control);
            }
            Widget::Tasks(w) => {
                w.on_control(control);
                w.on_select_task(|id| with_app(|a| a.task_action(i64::from(id), "select")));
            }
            Widget::Mini(w) => {
                w.on_control(control);
            }
        }
        widget.window().on_close_requested(move || {
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|a| {
                    a.preferences[index].enabled = false;
                    a.destroy_widget(index);
                    a.save_widget(index);
                    a.refresh_widget_rows();
                    a.reschedule();
                })
            });
            slint::CloseRequestResponse::KeepWindowShown
        });
        if let Err(e) = widget.window().show() {
            self.preferences[index].enabled = false;
            self.message(&e.to_string());
            return;
        }
        self.widgets[index] = Some(widget);
        self.widget_attached[index] = false;
        self.apply_appearance();
        self.position_widget(index);
        if let Some(Widget::Tasks(widget)) = &self.widgets[index] {
            widget.set_tasks(self.task_models.today.clone().into());
        }
        self.render();
        self.request_today();
        slint::Timer::single_shot(Duration::from_millis(60), || {
            with_app(|a| a.attach_windows())
        });
    }
    fn destroy_widget(&mut self, index: usize) {
        if let Some(widget) = self.widgets[index].take() {
            let position = widget.window().position();
            self.preferences[index].x = Some(position.x);
            self.preferences[index].y = Some(position.y);
            if let Some(h) = window_handle(widget.window()) {
                platform_win::detach_window(h);
            }
            let _ = widget.window().hide();
        }
        self.widget_attached[index] = false;
    }
    fn attach_windows(&mut self) {
        if !self.main_attached
            && let Some(h) = window_handle(self.main.window())
        {
            platform_win::attach_window(h, None, native_handler());
            self.main_attached = true;
            self.fit_main();
            self.apply_appearance();
            if self.main_requested_visible {
                platform_win::foreground(h);
            }
        }
        for index in 0..3 {
            if !self.widget_attached[index]
                && let Some(w) = &self.widgets[index]
                && let Some(h) = window_handle(w.window())
            {
                platform_win::attach_window(h, Some(index), native_handler());
                self.widget_attached[index] = true;
                self.position_widget(index);
                self.apply_widget(index);
            }
        }
    }
    fn position_widget(&self, index: usize) {
        if let Some(widget) = &self.widgets[index] {
            let p = &self.preferences[index];
            let size = widget.window().size();
            let (x, y) = platform_win::clamp_position(
                p.x.unwrap_or(40 + index as i32 * 330),
                p.y.unwrap_or(80),
                size.width as i32,
                size.height as i32,
            );
            widget
                .window()
                .set_position(slint::PhysicalPosition::new(x, y));
        }
    }
    fn apply_widget(&self, index: usize) {
        if let Some(widget) = &self.widgets[index]
            && let Some(h) = window_handle(widget.window())
        {
            platform_win::apply_widget(h, &self.preferences[index]);
        }
    }
    fn save_widget(&self, index: usize) {
        self.submit(
            Operation::Widget {
                index,
                preferences: self.preferences[index].clone(),
            },
            0,
        );
    }
    fn reset_widget(&mut self, index: usize) {
        if index >= 3 {
            return;
        }
        self.preferences[index].x = None;
        self.preferences[index].y = None;
        self.preferences[index].click_through = false;
        self.position_widget(index);
        self.apply_widget(index);
        self.save_widget(index);
        self.refresh_widget_rows();
    }
    fn refresh_widget_rows(&self) {
        let titles = [
            ("Timer ring", "Countdown and session controls."),
            ("Today's tasks", "Active tasks and today's focus totals."),
            ("Mini timer", "Compact countdown with start and pause."),
        ];
        self.main.set_widgets(ModelRc::new(VecModel::from(
            self.preferences
                .iter()
                .zip(titles)
                .map(|(p, (title, description))| WidgetRow {
                    title: title.into(),
                    description: description.into(),
                    enabled: p.enabled,
                    layer: p.layer,
                    click_through: p.click_through,
                })
                .collect::<Vec<_>>(),
        )));
    }
    fn show(&mut self) {
        self.main_requested_visible = true;
        if let Err(e) = self.main.show() {
            self.message(&e.to_string());
            return;
        }
        self.main.window().set_minimized(false);
        if let Some(h) = window_handle(self.main.window()) {
            platform_win::foreground(h);
        }
        self.command(Command::Tick);
        self.attach_windows();
        self.fit_main();
    }
    fn native_event(&mut self, event: Event) {
        if self.quitting {
            return;
        }
        match event {
            Event::Show => self.show(),
            Event::Toggle => self.command(Command::Toggle),
            Event::Skip => self.command(Command::Skip),
            Event::Reset => self.command(Command::Reset),
            Event::Quit => self.quit(),
            Event::Shutdown => {
                self.main.set_data_busy(false);
                self.cancel_capture();
                self.load_settings();
                self.quit();
            }
            Event::Suspend => {
                self.command(Command::Suspend);
            }
            Event::Resume => self.command(Command::Resume),
            Event::DisplayChanged => {
                for i in 0..3 {
                    self.position_widget(i);
                }
                self.fit_main();
            }
            Event::VisibilityChanged => {
                self.render();
                self.reschedule();
            }
            Event::AppearanceChanged => self.apply_appearance(),
            Event::ToggleWidget(index) => {
                self.preferences[index].enabled = !self.preferences[index].enabled;
                if self.preferences[index].enabled {
                    self.create_widget(index);
                } else {
                    self.destroy_widget(index);
                }
                self.save_widget(index);
                self.refresh_widget_rows();
                self.reschedule();
            }
            Event::UnlockWidgets => {
                for i in 0..3 {
                    self.preferences[i].click_through = false;
                    self.apply_widget(i);
                    self.save_widget(i);
                }
                self.refresh_widget_rows();
            }
            Event::ResetWidgets => {
                for i in 0..3 {
                    self.reset_widget(i);
                }
            }
            Event::WidgetMoved { index, x, y } => {
                self.preferences[index].x = Some(x);
                self.preferences[index].y = Some(y);
                self.save_widget(index);
            }
            Event::Error(error) => self.message(&error),
            Event::FileDialog { kind, result } => self.file_selected(kind, result),
        }
    }
    fn quit(&mut self) {
        self.cancel_capture();
        if self.main.get_data_busy() {
            self.message("Finish the file operation before quitting.");
            return;
        }
        if self.main.get_settings_dirty() {
            self.main.set_confirm_quit(true);
            self.show();
            return;
        }
        if self.quitting {
            return;
        }
        let now = WindowsClock.now();
        let previous = self.timer.clone();
        let effects = self.timer.dispatch(Command::Pause, now);
        let result = self
            .storage
            .finish(effects, self.timer.checkpoint(now), |result| {
                let _ =
                    slint::invoke_from_event_loop(move || with_app(|app| app.finish_quit(result)));
            });
        if let Err(error) = result {
            self.timer = previous;
            self.message(&format!("Could not queue the final save: {error}"));
            self.reschedule();
            return;
        }
        self.quitting = true;
        self.display_timer.stop();
        self.expiry_timer.stop();
        self.checkpoint_timer.stop();
        self.calendar_timer.stop();
        self.render();
        self.message("Saving before exiting…");
    }
    fn finish_quit(&mut self, result: focus_storage::Result<Response>) {
        if let Err(e) = result {
            self.quitting = false;
            self.message(&format!("Could not save before exiting: {e}"));
            self.show();
            return;
        }
        for i in 0..3 {
            self.destroy_widget(i);
        }
        self.platform.stop();
        let _ = self.main.hide();
        let _ = slint::quit_event_loop();
    }
    fn apply_appearance(&self) {
        let high_contrast = platform_win::high_contrast();
        let reduced_motion = platform_win::reduced_motion();
        let dark = self.timer.settings.appearance == 2
            || (self.timer.settings.appearance == 0 && platform_win::prefers_dark());
        let colors = platform_win::system_colors().map(|color| {
            slint::Color::from_rgb_u8(color as u8, (color >> 8) as u8, (color >> 16) as u8)
        });
        let apply = |tokens: Tokens<'_>| {
            tokens.set_reduced_motion(
                self.timer.settings.reduced_motion || high_contrast || reduced_motion,
            );
            tokens.set_high_contrast(high_contrast);
            tokens.set_dark(dark);
            tokens.set_system_background(colors[0]);
            tokens.set_system_foreground(colors[1]);
            tokens.set_system_highlight(colors[2]);
            tokens.set_system_highlight_text(colors[3]);
            tokens.set_system_muted(colors[4]);
        };
        apply(self.main.global::<Tokens>());
        for widget in self.widgets.iter().flatten() {
            match widget {
                Widget::Timer(w) => apply(w.global::<Tokens>()),
                Widget::Tasks(w) => apply(w.global::<Tokens>()),
                Widget::Mini(w) => apply(w.global::<Tokens>()),
            }
        }
        if let Some(h) = window_handle(self.main.window()) {
            self.main.set_mica_active(platform_win::configure_main(
                h,
                self.timer.settings.mica && !high_contrast,
            ));
        }
        self.render();
    }
    fn sync_layout(&self) {
        let window = self.main.window();
        let size = window.size().to_logical(window.scale_factor());
        self.main
            .set_compact(size.width < 840.0 || size.height < 650.0);
    }
    fn fit_main(&self) {
        let window = self.main.window();
        let position = window.position();
        let size = window.size();
        let (_, _, width, height) = platform_win::work_area(position.x, position.y);
        let fitted = slint::PhysicalSize::new(
            size.width.min(width.max(1) as u32),
            size.height.min(height.max(1) as u32),
        );
        if fitted != size {
            window.set_size(fitted);
        }
        let (x, y) = platform_win::clamp_position(
            position.x,
            position.y,
            fitted.width as i32,
            fitted.height as i32,
        );
        window.set_position(slint::PhysicalPosition::new(x, y));
    }
}
fn duration_label(ms: u64) -> String {
    let minutes = ms / 60_000;
    if minutes >= 60 {
        format!("{}h {:02}m", minutes / 60, minutes % 60)
    } else if ms > 0 && minutes == 0 {
        "<1m".into()
    } else {
        format!("{minutes}m")
    }
}
fn ring_path(fraction: f32) -> String {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0001 {
        return String::new();
    }
    if fraction >= 0.9999 {
        return "M120 5 A115 115 0 1 1 120 235 A115 115 0 1 1 120 5".into();
    }
    let angle = std::f32::consts::TAU * fraction - std::f32::consts::FRAC_PI_2;
    format!(
        "M120 5 A115 115 0 {} 1 {:.3} {:.3}",
        if fraction > 0.5 { 1 } else { 0 },
        120.0 + 115.0 * angle.cos(),
        120.0 + 115.0 * angle.sin()
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ring_handles_zero_full_and_half() {
        assert!(ring_path(0.0).is_empty());
        assert!(ring_path(1.0).contains("120 235"));
        assert!(ring_path(0.5).contains("120.000 235.000"));
    }
}
