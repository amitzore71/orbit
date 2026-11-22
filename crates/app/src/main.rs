#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use focus_core::{Accent, Density, Mode, MotionLevel, Settings};
use orbit_runtime::{
    AppCommand, AppSnapshot, Reply, ReportQuery, RuntimeError, RuntimeEvent, RuntimeHandle,
    TimerAction, TimerSnapshot,
};
use platform_win::{Event, Platform, TaskbarProgressState, WindowsClock};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const WEBVIEW_ARGS: &str = "--renderer-process-limit=1 --disable-background-networking --disable-component-update --disable-domain-reliability --disable-sync --disable-translate --disable-extensions --disable-features=msSmartScreenProtection,msWebOOUI,msPdfOOUI,Translate,OptimizationHints,MediaRouter,InterestFeedContentSuggestions,msEdgeCollections";

struct Desktop {
    runtime: RuntimeHandle,
    platform: Platform,
    quitting: AtomicBool,
    settings_dirty: AtomicBool,
    main_timer_visible: AtomicBool,
    main_maximized: AtomicBool,
    mica_active: AtomicBool,
    widgets_revision: AtomicU64,
    file_busy: AtomicBool,
    pending_restore: Mutex<Option<PathBuf>>,
    settings: Mutex<Settings>,
    last_tooltip: Mutex<String>,
    last_taskbar: Mutex<(TaskbarProgressState, u64)>,
    last_timer: Mutex<Option<TimerSnapshot>>,
    active_task_title: Mutex<Option<String>>,
    startup_probe: Option<PathBuf>,
    started: std::time::Instant,
    ready_recorded: AtomicBool,
    webview_dir: PathBuf,
}
#[derive(Clone, Serialize)]
struct Appearance {
    dark: bool,
    high_contrast: bool,
    reduced_motion: bool,
    mica_active: bool,
    accent: Accent,
    density: Density,
    motion_level: MotionLevel,
    system_dark: bool,
    system_reduced_motion: bool,
}
#[derive(Clone, Serialize)]
struct WindowState {
    maximized: bool,
}
#[derive(Clone, Serialize)]
struct Change {
    revision: u64,
    domains: Vec<&'static str>,
}
fn error(message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        accepted: false,
    }
}
fn snapshot(runtime: &RuntimeHandle) -> Result<AppSnapshot, RuntimeError> {
    match runtime.call(AppCommand::Snapshot)? {
        Reply::Snapshot(s) => Ok(*s),
        _ => Err(error("Invalid runtime reply.")),
    }
}
fn native_handler(app: tauri::AppHandle) -> platform_win::EventHandler {
    Arc::new(move |event| {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || native_event(app, event));
    })
}
fn hwnd(window: &WebviewWindow) -> Option<isize> {
    window.hwnd().ok().map(|h| h.0 as isize)
}
fn format_clock(ms: u64) -> String {
    let seconds = ms.div_ceil(1000);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
fn sync_shell_progress(app: &tauri::AppHandle, timer: &TimerSnapshot) {
    let Some(state) = app.try_state::<Desktop>() else {
        return;
    };
    *state.last_timer.lock().unwrap_or_else(|e| e.into_inner()) = Some(timer.clone());
    let taskbar_enabled = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .taskbar_progress;
    let task_title = state
        .active_task_title
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    let tooltip = if timer.running {
        let clock = format_clock(timer.remaining_ms);
        match (timer.mode, task_title.as_deref()) {
            (Mode::Focus, Some(title)) if !title.is_empty() => {
                format!("Orbit · Focus {clock} — {title}")
            }
            _ => format!("Orbit · {} {clock}", timer.mode.label()),
        }
    } else if timer.started {
        format!("Orbit · Paused {}", format_clock(timer.remaining_ms))
    } else {
        "Orbit · Ready".to_string()
    };
    {
        let mut last = state.last_tooltip.lock().unwrap_or_else(|e| e.into_inner());
        if *last != tooltip {
            *last = tooltip.clone();
            state.platform.tooltip(&tooltip);
        }
    }
    let (tb_state, completed, total, bucket) =
        if !taskbar_enabled || !timer.started || timer.total_ms == 0 {
            (TaskbarProgressState::None, 0u64, 0u64, 0u64)
        } else {
            let elapsed = timer.total_ms.saturating_sub(timer.remaining_ms);
            let st = if timer.running {
                TaskbarProgressState::Normal
            } else {
                TaskbarProgressState::Paused
            };
            let permille = (elapsed.saturating_mul(1000) / timer.total_ms).min(1000);
            (st, elapsed, timer.total_ms, permille)
        };
    let should_update_taskbar = {
        let mut last = state.last_taskbar.lock().unwrap_or_else(|e| e.into_inner());
        if *last != (tb_state, bucket) {
            *last = (tb_state, bucket);
            true
        } else {
            false
        }
    };
    if should_update_taskbar {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(h) = handle.get_webview_window("main").and_then(|w| hwnd(&w)) {
                platform_win::set_taskbar_progress(h, tb_state, completed, total);
            }
        });
    }
}
fn active_title_from_snapshot(snap: &AppSnapshot) -> Option<String> {
    let id = snap
        .timer
        .bound_task
        .as_deref()
        .or(snap.timer.selected_task.as_deref())?;
    snap.tasks
        .iter()
        .find(|t| t.id == id && !t.deleted)
        .map(|t| t.title.clone())
}
fn appearance(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<Desktop>() else {
        return;
    };
    let settings = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let high = platform_win::high_contrast();
    let mica = app
        .get_webview_window("main")
        .and_then(|w| hwnd(&w))
        .is_some_and(|h| platform_win::configure_main(h, settings.mica && !high));
    state.mica_active.store(mica, Ordering::Release);
    let value = Appearance {
        dark: settings.appearance == 2
            || (settings.appearance == 0 && platform_win::prefers_dark()),
        high_contrast: high,
        reduced_motion: settings.reduced_motion || high || platform_win::reduced_motion(),
        mica_active: mica,
        accent: settings.accent,
        density: settings.density,
        motion_level: settings.effective_motion(),
        system_dark: platform_win::prefers_dark(),
        system_reduced_motion: high || platform_win::reduced_motion(),
    };
    let _ = app.emit("orbit://appearance", &value);
}
fn observers(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<Desktop>() {
        let count = app
            .webview_windows()
            .into_values()
            .filter(|w| {
                let visible = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(true);
                let _ = w.emit_to(w.label(), "orbit://visibility", visible);
                visible
                    && w.label() != "widget-1"
                    && (w.label() != "main" || s.main_timer_visible.load(Ordering::Acquire))
            })
            .count();
        s.runtime.set_timer_observers(count);
    }
}
fn emit_window_state(app: &tauri::AppHandle, window: &WebviewWindow, force: bool) {
    if window.label() != "main" {
        return;
    }
    let Some(state) = app.try_state::<Desktop>() else {
        return;
    };
    let maximized = window.is_maximized().unwrap_or(false);
    let changed = state.main_maximized.swap(maximized, Ordering::AcqRel) != maximized;
    if force || changed {
        let _ = window.emit_to("main", "orbit://window-state", WindowState { maximized });
    }
}
fn create_main_window(app: &tauri::AppHandle, visible: bool) -> Result<WebviewWindow, String> {
    let webview_dir = app.state::<Desktop>().webview_dir.clone();
    let main = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .data_directory(webview_dir)
        .additional_browser_args(WEBVIEW_ARGS)
        .transparent(true)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .title("Orbit · Local focus workspace")
        .inner_size(1160., 790.)
        .min_inner_size(760., 540.)
        .resizable(true)
        .maximizable(true)
        .decorations(false)
        .visible(visible)
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(h) = hwnd(&main) {
        platform_win::attach_window(h, None, native_handler(app.clone()));
    }
    Ok(main)
}
fn show(app: &tauri::AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let window = handle.get_webview_window("main").or_else(|| {
            create_main_window(&handle, true)
                .map_err(|e| handle.emit("orbit://error", error(e)))
                .ok()
        });
        if let Some(w) = window {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
            if let Some(h) = hwnd(&w) {
                platform_win::foreground(h);
            }
        }
        appearance(&handle);
        observers(&handle);
        let _ = handle.emit("orbit://resync", ());
    });
}
fn sync_widgets(app: &tauri::AppHandle, snapshot: AppSnapshot) {
    let webview_dir = app.state::<Desktop>().webview_dir.clone();
    let app = app.clone();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = handle.state::<Desktop>();
        if snapshot.revision
            < state
                .widgets_revision
                .fetch_max(snapshot.revision, Ordering::AcqRel)
        {
            return;
        }
        for (i, p) in snapshot.widgets.iter().enumerate() {
            let label = format!("widget-{i}");
            if !p.enabled {
                if let Some(w) = handle.get_webview_window(&label) {
                    if let Some(h) = hwnd(&w) {
                        platform_win::detach_window(h);
                    }
                    let _ = w.destroy();
                }
                continue;
            }
            let existing = handle.get_webview_window(&label);
            let fresh = existing.is_none();
            let window = existing.or_else(|| {
                let (width, height) = match i {
                    0 => (300., 360.),
                    1 => (340., 390.),
                    _ => (310., 116.),
                };
                WebviewWindowBuilder::new(
                    &handle,
                    &label,
                    WebviewUrl::App(format!("widget/index.html?kind={i}").into()),
                )
                .data_directory(webview_dir.clone())
                .additional_browser_args(WEBVIEW_ARGS)
                .background_color(tauri::window::Color(18, 20, 23, 255))
                .title(format!(
                    "Orbit · {}",
                    ["Timer ring", "Today's tasks", "Mini timer"][i]
                ))
                .inner_size(width, height)
                .decorations(false)
                .resizable(false)
                .skip_taskbar(true)
                .visible(false)
                .build()
                .map_err(|e| handle.emit("orbit://error", error(e.to_string())))
                .ok()
            });
            if let Some(w) = window {
                if let Some(h) = hwnd(&w) {
                    if fresh {
                        platform_win::attach_window(h, Some(i), native_handler(handle.clone()));
                    }
                    platform_win::apply_widget(h, p);
                }
                let size = w.outer_size().unwrap_or(tauri::PhysicalSize::new(300, 360));
                let (x, y) = platform_win::clamp_position(
                    p.x.unwrap_or(40 + i as i32 * 330),
                    p.y.unwrap_or(80),
                    size.width as i32,
                    size.height as i32,
                );
                if fresh || w.outer_position().is_ok_and(|pos| pos.x != x || pos.y != y) {
                    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
                }
                if fresh {
                    let _ = w.show();
                }
            }
        }
        observers(&handle);
        appearance(&handle);
    });
}
fn quit(app: &tauri::AppHandle, shutdown: bool) {
    let Some(s) = app.try_state::<Desktop>() else {
        return;
    };
    if s.file_busy.load(Ordering::Acquire) && !shutdown {
        let _ = app.emit(
            "orbit://error",
            error("Finish the file operation before quitting."),
        );
        show(app);
        return;
    }
    if s.settings_dirty.load(Ordering::Acquire) && !shutdown {
        show(app);
        let _ = app.emit("orbit://confirm-quit", ());
        return;
    }
    if s.quitting.swap(true, Ordering::AcqRel) {
        return;
    }
    match s.runtime.call(AppCommand::Finish) {
        Ok(_) => {
            s.platform.stop();
            app.exit(0);
        }
        Err(e) => {
            s.quitting.store(false, Ordering::Release);
            let _ = app.emit("orbit://error", e);
            show(app);
        }
    }
}
fn native_event(app: tauri::AppHandle, event: Event) {
    let Some(s) = app.try_state::<Desktop>() else {
        return;
    };
    if s.file_busy.load(Ordering::Acquire)
        && matches!(event, Event::Toggle | Event::Skip | Event::Reset)
    {
        let _ = app.emit(
            "orbit://error",
            error("Finish the file operation before changing the timer."),
        );
        return;
    }
    let command = match event {
        Event::Show => {
            show(&app);
            None
        }
        Event::Toggle => Some(AppCommand::Timer {
            action: TimerAction::Toggle,
        }),
        Event::Skip => Some(AppCommand::Timer {
            action: TimerAction::Skip,
        }),
        Event::Reset => Some(AppCommand::Timer {
            action: TimerAction::Reset,
        }),
        Event::Suspend => Some(AppCommand::Timer {
            action: TimerAction::Suspend,
        }),
        Event::Resume => Some(AppCommand::Timer {
            action: TimerAction::Resume,
        }),
        Event::Quit => {
            quit(&app, false);
            None
        }
        Event::Shutdown => {
            quit(&app, true);
            None
        }
        Event::VisibilityChanged => {
            observers(&app);
            None
        }
        Event::AppearanceChanged => {
            appearance(&app);
            None
        }
        Event::DisplayChanged => {
            if let Ok(v) = snapshot(&s.runtime) {
                sync_widgets(&app, v);
            }
            None
        }
        Event::WidgetMoved { index, x, y } => Some(AppCommand::MoveWidget { index, x, y }),
        Event::ToggleWidget(index) => {
            if let Ok(v) = snapshot(&s.runtime) {
                if let Some(p) = v.widgets.get(index) {
                    let mut p = p.clone();
                    p.enabled = !p.enabled;
                    let _ = s.runtime.call(AppCommand::Widget {
                        index,
                        preferences: p,
                    });
                }
            }
            None
        }
        Event::UnlockWidgets | Event::ResetWidgets => {
            let reset = matches!(event, Event::ResetWidgets);
            if let Ok(v) = snapshot(&s.runtime) {
                for (index, mut p) in v.widgets.into_iter().enumerate() {
                    p.click_through = false;
                    if reset {
                        p.x = None;
                        p.y = None;
                    }
                    let _ = s.runtime.call(AppCommand::Widget {
                        index,
                        preferences: p,
                    });
                }
            }
            None
        }
        Event::Error(message) => {
            let _ = app.emit("orbit://error", error(message));
            None
        }
        Event::FileDialog { .. } => None,
    };
    if let Some(c) = command {
        if let Err(e) = s.runtime.call(c) {
            let _ = app.emit("orbit://error", e);
        }
    }
}
fn allowed(window: &WebviewWindow, command: &AppCommand) -> bool {
    if matches!(
        command,
        AppCommand::Backup { .. }
            | AppCommand::Restore { .. }
            | AppCommand::Export { .. }
            | AppCommand::Finish
    ) {
        return false;
    }
    window.label() == "main"
        || (window.label() == "widget-1"
            && matches!(
                command,
                AppCommand::Task { .. }
                    | AppCommand::CreateTask { .. }
                    | AppCommand::EditTask { .. }
            ))
        || (window.label().starts_with("widget-")
            && matches!(
                command,
                AppCommand::Snapshot
                    | AppCommand::Timer {
                        action: TimerAction::Toggle
                            | TimerAction::Start
                            | TimerAction::Pause
                            | TimerAction::Skip
                            | TimerAction::Reset
                    }
                    | AppCommand::StartTask { .. }
                    | AppCommand::SelectTask { .. }
            ))
}
#[tauri::command]
async fn orbit_command(
    app: tauri::AppHandle,
    window: WebviewWindow,
    command: AppCommand,
) -> Result<Reply, RuntimeError> {
    if !allowed(&window, &command) {
        return Err(error("This action is unavailable in this window."));
    }
    if app.state::<Desktop>().file_busy.load(Ordering::Acquire)
        && matches!(
            command,
            AppCommand::Timer {
                action: TimerAction::Start
                    | TimerAction::Toggle
                    | TimerAction::Skip
                    | TimerAction::Reset
            } | AppCommand::ExtendTimer { .. }
                | AppCommand::SelectMode { .. }
                | AppCommand::SelectTask { .. }
                | AppCommand::StartTask { .. }
                | AppCommand::CreateTask { start: true, .. }
        )
    {
        return Err(error(
            "Finish the file operation before changing the timer.",
        ));
    }
    if let AppCommand::SaveSettings { settings } = &command {
        platform_win::validate_hotkeys(settings).map_err(error)?;
    }
    let runtime = app.state::<Desktop>().runtime.clone();
    tauri::async_runtime::spawn_blocking(move || runtime.call(command))
        .await
        .map_err(|e| error(e.to_string()))?
}
#[tauri::command]
async fn window_command(
    app: tauri::AppHandle,
    window: WebviewWindow,
    action: String,
) -> Result<(), RuntimeError> {
    match action.as_str() {
        "minimize" => window.minimize().map_err(|e| error(e.to_string()))?,
        "toggle_maximize" if window.label() == "main" => {
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize().map_err(|e| error(e.to_string()))?;
            } else {
                window.maximize().map_err(|e| error(e.to_string()))?;
            }
            emit_window_state(&app, &window, true);
            observers(&app);
        }
        "maximize" if window.label() == "main" => {
            window.maximize().map_err(|e| error(e.to_string()))?;
            emit_window_state(&app, &window, true);
            observers(&app);
        }
        "unmaximize" if window.label() == "main" => {
            window.unmaximize().map_err(|e| error(e.to_string()))?;
            emit_window_state(&app, &window, true);
            observers(&app);
        }
        "close" | "hide" => {
            if window.label() == "main" {
                window.hide().map_err(|e| error(e.to_string()))?;
            } else {
                let index = window
                    .label()
                    .strip_prefix("widget-")
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| error("Invalid widget."))?;
                let h = app.state::<Desktop>().runtime.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let s = snapshot(&h)?;
                    let mut p = s.widgets[index].clone();
                    p.enabled = false;
                    h.call(AppCommand::Widget {
                        index,
                        preferences: p,
                    })
                })
                .await
                .map_err(|e| error(e.to_string()))??;
            }
            observers(&app);
        }
        "show" => show(&app),
        "quit" | "discard_and_quit" if window.label() == "main" => {
            if action == "discard_and_quit" {
                app.state::<Desktop>()
                    .settings_dirty
                    .store(false, Ordering::Release);
            }
            let a = app.clone();
            tauri::async_runtime::spawn_blocking(move || quit(&a, false));
        }
        "data_folder" if window.label() == "main" => {
            let rt = app.state::<Desktop>().runtime.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let s = snapshot(&rt)?;
                if let Some(p) = std::path::Path::new(&s.data_location).parent() {
                    platform_win::open_folder(p);
                }
                Ok::<_, RuntimeError>(())
            })
            .await
            .map_err(|e| error(e.to_string()))??;
        }
        "register_notifications" if window.label() == "main" => {
            platform_win::register_notifications().map_err(error)?;
            app.state::<Desktop>()
                .platform
                .notify("Orbit notifications", "Your focus alerts are ready.");
        }
        "preview_sound" if window.label() == "main" => {
            let s = app
                .state::<Desktop>()
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            platform_win::play_alarm(s.sound, s.volume, native_handler(app.clone()));
        }
        _ => return Err(error("Unknown window action.")),
    }
    Ok(())
}
#[tauri::command]
fn set_settings_dirty(
    app: tauri::AppHandle,
    window: WebviewWindow,
    dirty: bool,
) -> Result<(), RuntimeError> {
    if window.label() != "main" {
        return Err(error("Only the main window has settings drafts."));
    }
    app.state::<Desktop>()
        .settings_dirty
        .store(dirty, Ordering::Release);
    Ok(())
}
#[tauri::command]
fn set_timer_visibility(app: tauri::AppHandle, window: WebviewWindow, visible: bool) {
    if window.label() == "main" {
        app.state::<Desktop>()
            .main_timer_visible
            .store(visible, Ordering::Release);
        observers(&app);
    }
}
#[tauri::command]
async fn capture_shortcuts(
    app: tauri::AppHandle,
    window: WebviewWindow,
    capturing: bool,
) -> Result<(), RuntimeError> {
    if window.label() != "main" {
        return Err(error("Unavailable in this window."));
    }
    let platform = app.state::<Desktop>().platform.clone();
    let settings = app
        .state::<Desktop>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let s = if capturing {
            Settings {
                hotkey_show: String::new(),
                hotkey_toggle: String::new(),
                hotkey_skip: String::new(),
                ..settings
            }
        } else {
            settings
        };
        let errors = platform.hotkeys(&s);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(error(errors.join(" ")))
        }
    })
    .await
    .map_err(|e| error(e.to_string()))?
}
#[tauri::command]
fn get_window_state(window: WebviewWindow) -> WindowState {
    WindowState {
        maximized: window.is_maximized().unwrap_or(false),
    }
}
#[tauri::command]
fn get_appearance(app: tauri::AppHandle) -> Appearance {
    let s = app
        .state::<Desktop>()
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let high = platform_win::high_contrast();
    Appearance {
        dark: s.appearance == 2 || (s.appearance == 0 && platform_win::prefers_dark()),
        high_contrast: high,
        reduced_motion: s.reduced_motion || high || platform_win::reduced_motion(),
        mica_active: app.state::<Desktop>().mica_active.load(Ordering::Acquire),
        accent: s.accent,
        density: s.density,
        motion_level: s.effective_motion(),
        system_dark: platform_win::prefers_dark(),
        system_reduced_motion: high || platform_win::reduced_motion(),
    }
}
#[tauri::command]
fn frontend_ready(app: tauri::AppHandle, window: WebviewWindow) {
    let state = app.state::<Desktop>();
    if window.label() == "main" && !state.ready_recorded.swap(true, Ordering::AcqRel) {
        if let Some(path) = &state.startup_probe {
            let _=std::fs::write(path,serde_json::json!({"interactiveMs":state.started.elapsed().as_secs_f64()*1000.0,"scope":"React workspace mounted after initial Rust snapshot; excludes physical display latency"}).to_string());
        }
    }
}
#[tauri::command]
async fn file_operation(
    app: tauri::AppHandle,
    window: WebviewWindow,
    kind: u32,
    query: Option<ReportQuery>,
) -> Result<Option<String>, RuntimeError> {
    if window.label() != "main" || kind > 2 {
        return Err(error("Invalid file operation."));
    }
    if app
        .state::<Desktop>()
        .file_busy
        .swap(true, Ordering::AcqRel)
    {
        return Err(error("A file operation is already open."));
    }
    let app_clone = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state = app_clone.state::<Desktop>();
        state.runtime.call(AppCommand::Timer {
            action: if kind == 2 {
                TimerAction::Checkpoint
            } else {
                TimerAction::Pause
            },
        })?;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        platform_win::choose_file(
            kind,
            hwnd(&window),
            Arc::new(move |e| {
                if let Event::FileDialog { result, .. } = e {
                    let _ = tx.send(result);
                }
            }),
        );
        let path = rx
            .recv()
            .map_err(|e| error(e.to_string()))?
            .map_err(error)?;
        let Some(path) = path else {
            return Ok(None);
        };
        if kind == 1 {
            let text = path.display().to_string();
            *state
                .pending_restore
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(path);
            return Ok(Some(text));
        }
        let command = if kind == 0 {
            AppCommand::Backup {
                path: path.display().to_string(),
            }
        } else {
            AppCommand::Export {
                path: path.display().to_string(),
                query: query.unwrap_or_default(),
            }
        };
        state.runtime.call(command)?;
        Ok(Some(path.display().to_string()))
    })
    .await
    .map_err(|e| error(e.to_string()));
    app.state::<Desktop>()
        .file_busy
        .store(false, Ordering::Release);
    result?
}
#[tauri::command]
async fn confirm_restore(
    app: tauri::AppHandle,
    window: WebviewWindow,
    confirm: bool,
) -> Result<(), RuntimeError> {
    if window.label() != "main" {
        return Err(error("Unavailable in this window."));
    }
    let path = app
        .state::<Desktop>()
        .pending_restore
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    if !confirm {
        return Ok(());
    }
    let path = path.ok_or_else(|| error("Choose a backup first."))?;
    if app
        .state::<Desktop>()
        .file_busy
        .swap(true, Ordering::AcqRel)
    {
        return Err(error("A file operation is already running."));
    }
    let runtime = app.state::<Desktop>().runtime.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        runtime.call(AppCommand::Restore {
            path: path.display().to_string(),
        })
    })
    .await
    .map_err(|e| error(e.to_string()));
    app.state::<Desktop>()
        .file_busy
        .store(false, Ordering::Release);
    result??;
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            platform_win::message_box("Orbit could not start", &e.to_string());
            std::process::ExitCode::FAILURE
        }
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|a| a == "--register-notifications") {
        platform_win::register_notifications()?;
        return Ok(());
    }
    let data_dir = args
        .iter()
        .position(|a| a == "--data-dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(platform_win::data_directory);
    let instance_name = if args.iter().any(|a| a == "--isolated") {
        format!("Local\\StillPomodoro.{}", std::process::id())
    } else {
        "Local\\StillPomodoro.Desktop.v1".into()
    };
    let Some(_instance) = platform_win::Instance::acquire(&instance_name)? else {
        return Ok(());
    };
    let args_for_setup = args.clone();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            orbit_command,
            window_command,
            set_settings_dirty,
            set_timer_visibility,
            capture_shortcuts,
            get_window_state,
            get_appearance,
            frontend_ready,
            file_operation,
            confirm_restore
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let events_handle = handle.clone();
            let runtime = orbit_runtime::start(
                platform_win::database_path(&data_dir),
                Arc::new(WindowsClock),
                Arc::new(move |event| match event {
                    RuntimeEvent::Timer(s) => {
                        sync_shell_progress(&events_handle, &s);
                        // Hidden WebViews resync on reveal; do not wake their renderers every second.
                        for window in events_handle.webview_windows().into_values() {
                            if window.is_visible().unwrap_or(false)
                                && !window.is_minimized().unwrap_or(true)
                            {
                                let _ = window.emit_to(window.label(), "orbit://timer", &s);
                            }
                        }
                    }
                    RuntimeEvent::Changed { revision, domains } => {
                        let widgets = domains.contains(&"widgets");
                        let shell_sync = domains.contains(&"timer") || domains.contains(&"tasks");
                        let _ = events_handle.emit("orbit://changed", Change { revision, domains });
                        if widgets || shell_sync {
                            let app = events_handle.clone();
                            tauri::async_runtime::spawn_blocking(move || {
                                if let Some(s) = app.try_state::<Desktop>() {
                                    if let Ok(v) = snapshot(&s.runtime) {
                                        if shell_sync {
                                            *s.active_task_title
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner()) =
                                                active_title_from_snapshot(&v);
                                            sync_shell_progress(&app, &v.timer);
                                        }
                                        if widgets {
                                            sync_widgets(&app, v);
                                        }
                                    }
                                }
                            });
                        }
                    }
                    RuntimeEvent::Completed(mode) => {
                        if let Some(s) = events_handle.try_state::<Desktop>() {
                            let settings =
                                s.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
                            platform_win::play_alarm(
                                settings.sound,
                                settings.volume,
                                native_handler(events_handle.clone()),
                            );
                            if settings.notifications {
                                s.platform.notify(
                                    if mode == Mode::Focus {
                                        "Focus session complete"
                                    } else {
                                        "Break complete"
                                    },
                                    "Your next session is ready.",
                                );
                            }
                        }
                        let _ = events_handle.emit("orbit://completed", mode);
                    }
                    RuntimeEvent::Settings(settings) => {
                        let app = events_handle.clone();
                        if let Some(s) = app.try_state::<Desktop>() {
                            let mut saved = s.settings.lock().unwrap_or_else(|e| e.into_inner());
                            let old = saved.clone();
                            let mut warnings = s.platform.hotkeys(&settings);
                            if old.autostart != settings.autostart {
                                if let Err(e) = platform_win::set_autostart(settings.autostart) {
                                    warnings.push(e);
                                }
                            }
                            *saved = settings;
                            drop(saved);
                            let last_timer = s
                                .last_timer
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .clone();
                            if let Some(last_timer) = last_timer {
                                sync_shell_progress(&app, &last_timer);
                            }
                            if !warnings.is_empty() {
                                let _ = app.emit("orbit://error", error(warnings.join(" ")));
                            }
                            let a = app.clone();
                            let _ = app.run_on_main_thread(move || appearance(&a));
                        }
                    }
                    RuntimeEvent::Error(e) => {
                        let _ = events_handle.emit("orbit://error", e);
                    }
                }),
            )?;
            let initial = snapshot(&runtime).map_err(|e| e.message)?;
            let platform = Platform::start(native_handler(handle.clone()))?;
            let mut warnings = platform.hotkeys(&initial.settings);
            if initial.settings.autostart {
                if let Err(e) = platform_win::set_autostart(true) {
                    warnings.push(e);
                }
            }
            app.manage(Desktop {
                runtime: runtime.clone(),
                platform,
                quitting: AtomicBool::new(false),
                settings_dirty: AtomicBool::new(false),
                main_timer_visible: AtomicBool::new(true),
                main_maximized: AtomicBool::new(false),
                mica_active: AtomicBool::new(false),
                widgets_revision: AtomicU64::new(0),
                file_busy: AtomicBool::new(false),
                pending_restore: Mutex::new(None),
                settings: Mutex::new(initial.settings.clone()),
                last_tooltip: Mutex::new(String::new()),
                last_taskbar: Mutex::new((TaskbarProgressState::None, 0)),
                last_timer: Mutex::new(Some(initial.timer.clone())),
                active_task_title: Mutex::new(active_title_from_snapshot(&initial)),
                startup_probe: args_for_setup
                    .iter()
                    .any(|a| a == "--startup-probe")
                    .then(|| data_dir.join("startup.json")),
                started,
                ready_recorded: AtomicBool::new(false),
                webview_dir: data_dir.join("webview"),
            });
            let tray = args_for_setup.iter().any(|a| a == "--tray");
            // CLI sessions start before WebView creation; loading the UI never delays the timer.
            if args_for_setup.iter().any(|a| a == "--running") {
                let _ = runtime.call(AppCommand::Timer {
                    action: TimerAction::Start,
                });
            }
            // Lazy-create the main WebView only when visible (or when --startup-probe explicitly requests a mount check)
            // so normal --tray runs pure Rust with ~0 WebView RAM.
            let startup_probe = args_for_setup.iter().any(|a| a == "--startup-probe");
            if !tray || startup_probe {
                let main = create_main_window(&handle, !tray)?;
                if !tray && args_for_setup.iter().any(|a| a == "--minimized") {
                    let _ = main.minimize();
                }
            }
            sync_widgets(&handle, initial);
            if args_for_setup.iter().any(|a| a == "--all-widgets") {
                let h = runtime.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Ok(v) = snapshot(&h) {
                        for (index, mut p) in v.widgets.into_iter().enumerate() {
                            p.enabled = true;
                            let _ = h.call(AppCommand::Widget {
                                index,
                                preferences: p,
                            });
                        }
                    }
                });
            }
            if let Some(seconds) = args_for_setup
                .iter()
                .position(|a| a == "--quit-after")
                .and_then(|i| args_for_setup.get(i + 1))
                .and_then(|s| s.parse::<u64>().ok())
            {
                let a = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(seconds));
                    quit(&a, true);
                });
            }
            if !warnings.is_empty() {
                let _ = handle.emit("orbit://error", error(warnings.join(" ")));
            }
            observers(&handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if window.label() == "main" {
                        let _ = window.hide();
                        observers(app);
                    } else if let Some(i) = window
                        .label()
                        .strip_prefix("widget-")
                        .and_then(|v| v.parse::<usize>().ok())
                    {
                        let a = app.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            if let Some(s) = a.try_state::<Desktop>() {
                                if let Ok(v) = snapshot(&s.runtime) {
                                    let mut p = v.widgets[i].clone();
                                    p.enabled = false;
                                    let _ = s.runtime.call(AppCommand::Widget {
                                        index: i,
                                        preferences: p,
                                    });
                                }
                            }
                        });
                    }
                }
                tauri::WindowEvent::Resized(_) => {
                    if window.label() == "main" {
                        if let Some(w) = app.get_webview_window("main") {
                            emit_window_state(app, &w, false);
                        }
                    }
                    observers(app);
                }
                tauri::WindowEvent::Focused(_) => observers(app),
                _ => {}
            }
        })
        .build(tauri::generate_context!())?
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !app.state::<Desktop>().quitting.load(Ordering::Acquire) {
                    api.prevent_exit();
                    let a = app.clone();
                    tauri::async_runtime::spawn_blocking(move || quit(&a, false));
                }
            }
        });
    Ok(())
}
