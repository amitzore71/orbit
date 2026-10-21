use super::*;

#[test]
fn saving_curated_appearance_synchronizes_legacy_motion_and_is_durable() {
    use focus_core::{Accent, Density, MotionLevel};
    let (mut runtime, _, path) = fixture();
    runtime
        .execute(AppCommand::SaveSettings {
            settings: Settings {
                accent: Accent::Sage,
                density: Density::Compact,
                motion_level: Some(MotionLevel::Off),
                reduced_motion: false,
                ..Settings::default()
            },
        })
        .unwrap();
    assert!(runtime.timer.settings.reduced_motion);
    drop(runtime);
    let (_worker, loaded) = Worker::open(path).unwrap();
    assert_eq!(loaded.settings.accent, Accent::Sage);
    assert_eq!(loaded.settings.density, Density::Compact);
    assert_eq!(loaded.settings.effective_motion(), MotionLevel::Off);
    assert!(loaded.settings.reduced_motion);
}
use focus_core::{Settings, TimePoint};
use rusqlite::Connection;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> TimePoint {
        let n = self.0.load(Ordering::SeqCst);
        TimePoint {
            awake_ms: n,
            utc_ms: 1_792_000_000_000 + n as i64,
        }
    }
}
fn fixture() -> (Runtime, Arc<TestClock>, PathBuf) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir()
        .join(format!(
            "orbit-runtime-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ))
        .join("orbit.db");
    let (storage, loaded) = Worker::open(path.clone()).unwrap();
    let clock = Arc::new(TestClock::default());
    (
        Runtime::new(
            storage,
            loaded,
            path.clone(),
            clock.clone(),
            Arc::new(|_| {}),
        ),
        clock,
        path,
    )
}
fn create(r: &mut Runtime, title: &str, start: bool) -> String {
    match r
        .execute(AppCommand::CreateTask {
            title: title.into(),
            estimate: 2,
            today: true,
            start,
        })
        .unwrap()
    {
        Reply::Created(id) => id,
        _ => panic!("expected created task"),
    }
}
#[test]
fn task_ids_roundtrip_without_javascript_precision_loss() {
    let t = TaskDto::from(&focus_storage::Task {
        id: i64::MAX,
        ..Default::default()
    });
    let v = serde_json::to_value(t).unwrap();
    assert_eq!(v["id"], i64::MAX.to_string());
    let c: AppCommand =
        serde_json::from_value(serde_json::json!({"type":"start_task","id":"9223372036854775807"}))
            .unwrap();
    assert!(matches!(c, AppCommand::StartTask { .. }));
    assert!(
        serde_json::from_value::<AppCommand>(serde_json::json!({"type":"start_task","id":1}))
            .is_err()
    );
}
#[test]
fn starting_another_task_never_rebinds_the_session() {
    let (mut r, clock, _) = fixture();
    let a = create(&mut r, "First", true);
    let b = create(&mut r, "Next", false);
    clock.0.store(12_000, Ordering::SeqCst);
    r.execute(AppCommand::StartTask { id: b.clone() }).unwrap();
    let s = r.timer_snapshot();
    assert_eq!(s.bound_task, Some(a));
    assert_eq!(s.selected_task, Some(b));
    assert!(s.running);
}
#[test]
fn deletion_restore_and_planning_keep_ids_and_history() {
    let (mut r, _, _) = fixture();
    let id = create(&mut r, "Keep history", false);
    for action in [TaskAction::Delete, TaskAction::Restore] {
        r.execute(AppCommand::Task {
            id: id.clone(),
            action,
        })
        .unwrap();
    }
    let t = &r.snapshot().unwrap().tasks[0];
    assert_eq!(t.id, id);
    assert!(!t.deleted);
    assert!(t.planned_for.is_none());
}
#[test]
fn invalid_settings_do_not_change_the_runtime() {
    let (mut r, _, _) = fixture();
    let before = r.timer.settings.clone();
    let bad = Settings {
        focus_minutes: 0,
        ..before.clone()
    };
    assert!(
        r.execute(AppCommand::SaveSettings { settings: bad })
            .is_err()
    );
    assert_eq!(r.timer.settings, before);
}
#[test]
fn duration_changes_apply_to_the_next_started_session() {
    let (mut r, clock, _) = fixture();
    create(&mut r, "Current", true);
    clock.0.store(30_000, Ordering::SeqCst);
    r.execute(AppCommand::SaveSettings {
        settings: Settings {
            focus_minutes: 42,
            ..r.timer.settings.clone()
        },
    })
    .unwrap();
    assert_eq!(r.timer_snapshot().total_ms, 25 * 60_000);
    r.execute(AppCommand::Timer {
        action: TimerAction::Reset,
    })
    .unwrap();
    assert_eq!(r.timer_snapshot().total_ms, 42 * 60_000);
}
#[test]
fn accepted_storage_failure_pauses_and_retry_does_not_duplicate_creation() {
    let (mut r, _, path) = fixture();
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON tasks BEGIN SELECT RAISE(FAIL,'injected'); END;").unwrap();
    let e = r
        .execute(AppCommand::CreateTask {
            title: "Only once".into(),
            estimate: 1,
            today: true,
            start: false,
        })
        .unwrap_err();
    assert!(e.accepted);
    assert!(r.storage.is_blocked());
    assert!(!r.timer_snapshot().running);
    db.execute_batch("DROP TRIGGER fail_insert").unwrap();
    r.execute(AppCommand::RetrySaving).unwrap();
    assert_eq!(r.snapshot().unwrap().tasks.len(), 1);
    assert!(!r.timer_snapshot().running);
}
#[test]
fn final_save_and_reopen_restore_paused_without_absence_credit() {
    let (mut r, clock, path) = fixture();
    create(&mut r, "Durable", true);
    clock.0.store(90_000, Ordering::SeqCst);
    r.execute(AppCommand::Finish).unwrap();
    drop(r);
    let (storage, loaded) = Worker::open(path.clone()).unwrap();
    clock.0.store(1_000_000, Ordering::SeqCst);
    let reopened = Runtime::new(storage, loaded, path, clock, Arc::new(|_| {}));
    let s = reopened.timer_snapshot();
    assert!(!s.running);
    assert_eq!(s.remaining_ms, 25 * 60_000 - 90_000);
}
#[test]
fn checkpoint_events_do_not_invalidate_reports_recursively() {
    let (mut r, _, _) = fixture();
    let events = Arc::new(Mutex::new(Vec::new()));
    let out = events.clone();
    r.events = Arc::new(move |e| out.lock().unwrap().push(e));
    r.execute(AppCommand::Report {
        query: ReportQuery::default(),
    })
    .unwrap();
    assert!(
        !events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e,RuntimeEvent::Changed{domains,..} if domains.contains(&"reports")))
    );
}
#[test]
fn concurrent_callers_share_one_serial_runtime() {
    let (_, _, path) = fixture();
    let clock = Arc::new(TestClock::default());
    let handle = start(path, clock, Arc::new(|_| {})).unwrap();
    let joins = (0..8)
        .map(|i| {
            let h = handle.clone();
            thread::spawn(move || {
                h.call(AppCommand::CreateTask {
                    title: format!("Task {i}"),
                    estimate: 1,
                    today: true,
                    start: false,
                })
                .unwrap()
            })
        })
        .collect::<Vec<_>>();
    for j in joins {
        j.join().unwrap();
    }
    let Reply::Snapshot(s) = handle.call(AppCommand::Snapshot).unwrap() else {
        panic!()
    };
    assert_eq!(s.tasks.len(), 8);
    handle.call(AppCommand::Finish).unwrap();
}
#[test]
fn backup_restore_roundtrip_preserves_tasks_and_pauses_timer() {
    let (mut r, _, path) = fixture();
    let id = create(&mut r, "Original", true);
    let backup = path.with_file_name("manual.db");
    r.execute(AppCommand::Backup {
        path: backup.display().to_string(),
    })
    .unwrap();
    create(&mut r, "New", false);
    r.execute(AppCommand::Restore {
        path: backup.display().to_string(),
    })
    .unwrap();
    let s = r.snapshot().unwrap();
    assert_eq!(s.tasks.len(), 1);
    assert_eq!(s.tasks[0].id, id);
    assert!(!s.timer.running);
}

#[test]
fn retry_reloads_accepted_widget_preferences() {
    let (mut r, _, path) = fixture();
    let db = Connection::open(path).unwrap();
    db.execute_batch("CREATE TRIGGER fail_widget BEFORE INSERT ON meta WHEN NEW.key='widget0' BEGIN SELECT RAISE(FAIL,'injected'); END;").unwrap();
    // Use the actual key written by storage; inject a failure on any widget preference write.
    db.execute_batch("DROP TRIGGER fail_widget; CREATE TRIGGER fail_widget BEFORE INSERT ON meta WHEN NEW.key LIKE 'widget%' BEGIN SELECT RAISE(FAIL,'injected'); END;").unwrap();
    let p = WidgetPreferences {
        enabled: true,
        layer: 2,
        ..Default::default()
    };
    assert!(
        r.execute(AppCommand::Widget {
            index: 0,
            preferences: p
        })
        .is_err()
    );
    db.execute_batch("DROP TRIGGER fail_widget").unwrap();
    r.execute(AppCommand::RetrySaving).unwrap();
    let s = r.snapshot().unwrap();
    assert!(s.widgets[0].enabled);
    assert_eq!(s.widgets[0].layer, 2);
}

#[test]
fn exit_can_use_reserved_runtime_queue_capacity() {
    let (tx, rx) = mpsc::sync_channel(130);
    let h = RuntimeHandle {
        sender: tx,
        observers: Arc::new(AtomicUsize::new(0)),
        queued: Arc::new(AtomicUsize::new(128)),
    };
    assert!(h.call(AppCommand::Snapshot).is_err());
    let caller = thread::spawn(move || h.call(AppCommand::Finish));
    let request = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(!request.normal);
    assert!(matches!(request.command, AppCommand::Finish));
    request.reply.send(Ok(Reply::Saved(1))).unwrap();
    assert!(caller.join().unwrap().is_ok());
}

#[test]
fn blocked_resume_clears_suspension_before_a_later_start() {
    let (mut r, clock, path) = fixture();
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TRIGGER fail_task BEFORE INSERT ON tasks BEGIN SELECT RAISE(FAIL,'injected'); END;",
    )
    .unwrap();
    assert!(
        r.execute(AppCommand::CreateTask {
            title: "Recovery".into(),
            estimate: 1,
            today: true,
            start: false
        })
        .is_err()
    );
    r.execute(AppCommand::Timer {
        action: TimerAction::Suspend,
    })
    .unwrap();
    r.execute(AppCommand::Timer {
        action: TimerAction::Resume,
    })
    .unwrap();
    db.execute_batch("DROP TRIGGER fail_task").unwrap();
    r.store(Operation::Flush).unwrap();
    r.transition(Command::Start).unwrap();
    clock.0.store(1000, Ordering::SeqCst);
    r.transition(Command::Pause).unwrap();
    let credit: i64 = db
        .query_row(
            "SELECT COALESCE(SUM(active_ms),0) FROM intervals",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(credit, 1000);
}

#[test]
fn a_read_error_is_not_reported_as_a_retained_mutation() {
    let (mut r, _, _) = fixture();
    let error = r
        .execute(AppCommand::Report {
            query: ReportQuery {
                start: "invalid".into(),
                end: "invalid".into(),
                ..Default::default()
            },
        })
        .unwrap_err();
    assert!(!error.accepted);
    assert!(!r.storage.is_blocked());
}
