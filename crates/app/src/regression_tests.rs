use super::*;
use rusqlite::Connection;
use std::sync::mpsc;
struct Headless;
impl slint::platform::Platform for Headless {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(
            slint::platform::software_renderer::MinimalSoftwareWindow::new(
                slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
            ),
        )
    }
}
fn fixture() -> App {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/regressions")
        .join(format!(
            "{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    let (storage, loaded) = Worker::open(data_dir.join("orbit.db")).unwrap();
    let app = App {
        #[cfg(feature = "diagnostics")]
        diagnostic_ticks: 0,
        main: MainWindow::new().unwrap(),
        timer: Timer::new(loaded.settings),
        storage,
        platform: Platform::start(Arc::new(|_| {})).unwrap(),
        tasks: loaded.tasks,
        task_models: Default::default(),
        preferences: loaded.widgets,
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
        main_requested_visible: false,
    };
    app.load_settings();
    app
}
fn hold(worker: &Worker) -> mpsc::Sender<()> {
    let (release, wait) = mpsc::channel();
    let (ready, rx) = mpsc::channel();
    worker
        .submit(Operation::Tasks, move |_| {
            ready.send(()).unwrap();
            wait.recv().unwrap();
        })
        .unwrap();
    rx.recv().unwrap();
    release
}
#[test]
fn task_completion_preserves_pending_title_and_estimate_edits() {
    let mut app = fixture();
    let db = Connection::open(app.data_dir.join("orbit.db")).unwrap();
    db.execute(
        "INSERT INTO tasks(id,title,estimate) VALUES(1,'Original',1)",
        [],
    )
    .unwrap();
    app.tasks.push(Task {
        id: 1,
        title: "Original".into(),
        estimate: 1,
        completed: 0,
        done: false,
        ..Default::default()
    });
    let release = hold(&app.storage);
    app.main.set_edit_id(1);
    app.main.set_task_title("Edited title".into());
    app.main.set_task_estimate(3);
    app.save_task();
    app.task_action(1, "done");
    release.send(()).unwrap();
    app.storage.flush().unwrap();
    let task: (String, u32, bool) = db
        .query_row(
            "SELECT title,estimate,done FROM tasks WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(task, ("Edited title".into(), 3, true));
}
#[test]
fn rejected_settings_do_not_change_runtime_or_claim_success() {
    let mut app = fixture();
    let db = Connection::open(app.data_dir.join("orbit.db")).unwrap();
    db.execute_batch("CREATE TRIGGER deny_task BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let (tx, rx) = mpsc::channel();
    app.storage
        .submit(
            Operation::AddTask {
                title: "Accepted".into(),
                estimate: 1,
            },
            move |r| tx.send(r.is_err()).unwrap(),
        )
        .unwrap();
    assert!(rx.recv().unwrap());
    app.load_settings();
    app.main.set_focus_minutes(42);
    app.save_settings();
    assert_eq!(app.timer.settings.focus_minutes, 25);
    assert!(!app.main.get_message().contains("Settings saved"));
    db.execute_batch("DROP TRIGGER deny_task;").unwrap();
    app.storage.flush().unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM meta WHERE key='settings'", [], |r| {
            r.get::<_, u32>(0)
        })
        .unwrap(),
        0
    );
}
#[test]
fn quit_uses_reserved_capacity_and_preserves_the_final_pause() {
    let mut app = fixture();
    app.command(Command::Start);
    app.storage.flush().unwrap();
    let release = hold(&app.storage);
    for _ in 0..128 {
        app.storage.submit(Operation::Tasks, |_| {}).unwrap();
    }
    let started = std::time::Instant::now();
    app.quit();
    assert!(started.elapsed() < Duration::from_millis(100));
    assert!(app.quitting);
    release.send(()).unwrap();
    app.storage.flush().unwrap();
    let db = Connection::open(app.data_dir.join("orbit.db")).unwrap();
    let saved: String = db
        .query_row("SELECT value FROM meta WHERE key='timer'", [], |r| r.get(0))
        .unwrap();
    let checkpoint: focus_core::Checkpoint = serde_json::from_str(&saved).unwrap();
    assert!(!checkpoint.running);
    app.finish_quit(Ok(Response::Saved));
}
#[test]
fn task_snapshot_requests_coalesce_and_discard_superseded_results() {
    let mut app = fixture();
    let release = hold(&app.storage);
    for _ in 0..128 {
        app.storage_response(0, Ok(Response::TasksChanged));
    }
    assert!(app.tasks_in_flight && app.tasks_dirty);
    app.storage_response(
        TASK_REQUEST,
        Ok(Response::Tasks(vec![Task {
            id: 123,
            title: "Obsolete".into(),
            estimate: 1,
            completed: 0,
            done: false,
            ..Default::default()
        }])),
    );
    assert!(app.tasks.is_empty());
    assert!(app.tasks_in_flight && !app.tasks_dirty);
    release.send(()).unwrap();
    app.storage.flush().unwrap();
}
#[test]
fn resume_clears_suspension_even_while_storage_is_blocked() {
    let mut app = fixture();
    let db = Connection::open(app.data_dir.join("orbit.db")).unwrap();
    db.execute_batch("CREATE TRIGGER deny_task BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let (tx, rx) = mpsc::channel();
    app.storage
        .submit(
            Operation::AddTask {
                title: "Accepted".into(),
                estimate: 1,
            },
            move |r| tx.send(r.is_err()).unwrap(),
        )
        .unwrap();
    assert!(rx.recv().unwrap());
    app.timer.dispatch(Command::Suspend, WindowsClock.now());
    app.command(Command::Resume);
    db.execute_batch("DROP TRIGGER deny_task;").unwrap();
    app.storage.flush().unwrap();
    app.command(Command::Start);
    assert!(app.timer.snapshot(WindowsClock.now()).running);
    app.storage.flush().unwrap();
}

#[test]
fn settings_ack_preserves_newer_unsaved_edits_and_marks_the_right_baseline() {
    let mut app = fixture();
    app.main.set_focus_minutes(42);
    assert!(app.main.get_settings_dirty());
    app.apply_saved_settings(Settings {
        focus_minutes: 30,
        ..Settings::default()
    });
    assert_eq!(app.main.get_focus_minutes(), 42);
    assert_eq!(app.main.get_saved_settings().focus, 30);
    assert_eq!(
        app.timer.snapshot(WindowsClock.now()).remaining_ms,
        30 * 60_000
    );
    assert!(app.main.get_settings_dirty());
    app.load_settings();
    assert_eq!(app.main.get_focus_minutes(), 30);
    assert!(!app.main.get_settings_dirty());
}

#[test]
fn report_error_does_not_release_unrelated_pending_task_or_file_operations() {
    let mut app = fixture();
    app.main.set_task_saving(true);
    app.main.set_data_busy(true);
    app.start_after_create = true;
    app.storage_response(42, Err("Invalid report date".into()));
    assert!(app.main.get_task_saving());
    assert!(app.main.get_data_busy());
    assert!(app.start_after_create);
}

#[test]
fn recovery_ack_clears_an_accepted_create_draft_without_inserting_it_twice() {
    let mut app = fixture();
    let db = Connection::open(app.data_dir.join("orbit.db")).unwrap();
    db.execute_batch("CREATE TRIGGER deny_create BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    app.main.set_task_title("Keep exactly once".into());
    app.start_after_create = true;
    app.save_task();
    assert!(app.storage.flush().is_err());
    app.storage_response(CREATE_REQUEST, Err("injected".into()));
    assert!(app.create_pending);
    db.execute_batch("DROP TRIGGER deny_create").unwrap();
    app.storage.flush().unwrap();
    app.storage_response(0, Ok(Response::Flushed(Settings::default())));
    assert!(app.main.get_task_title().is_empty());
    assert!(!app.start_after_create);
    assert!(!app.timer.snapshot(WindowsClock.now()).running);
    app.save_task();
    app.storage.flush().unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn confirmed_windows_shutdown_saves_the_timer_without_waiting_on_a_settings_dialog() {
    let mut app = fixture();
    app.command(Command::Start);
    app.main.set_focus_minutes(42);
    assert!(app.main.get_settings_dirty());
    app.native_event(Event::Shutdown);
    assert!(app.quitting);
    app.storage.flush().unwrap();
    let db = Connection::open(app.data_dir.join("orbit.db")).unwrap();
    let saved: String = db
        .query_row("SELECT value FROM meta WHERE key='timer'", [], |r| r.get(0))
        .unwrap();
    let checkpoint: focus_core::Checkpoint = serde_json::from_str(&saved).unwrap();
    assert!(!checkpoint.running);
    assert_eq!(app.timer.settings.focus_minutes, 25);
}
