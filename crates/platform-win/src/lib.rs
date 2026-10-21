//! Small inbox-Windows integrations. No service, polling loop, or external runtime.
#![cfg(windows)]
use focus_core::{Clock, Settings, TimePoint, WidgetPreferences};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    mem::size_of,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{ToastNotification, ToastNotificationManager},
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        Media::Audio::*,
        Storage::EnhancedStorage::{PKEY_AppUserModel_ID, PKEY_AppUserModel_ToastActivatorCLSID},
        System::{
            Com::*, LibraryLoader::GetModuleHandleW, Registry::*, Threading::*,
            WindowsProgramming::QueryUnbiasedInterruptTimePrecise,
        },
        UI::{
            Accessibility::HIGHCONTRASTW, Input::KeyboardAndMouse::*, Shell::PropertiesSystem::*,
            Shell::*, WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, Interface, PCWSTR, PWSTR, w},
};

pub const APP_ID: &str = "Orbit.Pomodoro.Desktop";
const WINDOW_CLASS: PCWSTR = w!("StillPomodoro.MessageWindow.v1");
const WM_TRAY: u32 = WM_APP + 20;
const WM_COMMANDS: u32 = WM_APP + 21;
const WM_ACTIVATE_ORBIT: u32 = WM_APP + 22;
pub type Result<T> = std::result::Result<T, String>;

pub struct WindowsClock;
impl Clock for WindowsClock {
    fn now(&self) -> TimePoint {
        let awake_ms = unsafe { QueryUnbiasedInterruptTimePrecise() / 10_000 };
        let utc_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64;
        TimePoint { awake_ms, utc_ms }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Show,
    Toggle,
    Skip,
    Reset,
    Quit,
    Shutdown,
    ToggleWidget(usize),
    UnlockWidgets,
    ResetWidgets,
    Suspend,
    Resume,
    DisplayChanged,
    VisibilityChanged,
    AppearanceChanged,
    WidgetMoved {
        index: usize,
        x: i32,
        y: i32,
    },
    Error(String),
    FileDialog {
        kind: u32,
        result: Result<Option<PathBuf>>,
    },
}
pub type EventHandler = Arc<dyn Fn(Event) + Send + Sync>;

/// Read the Windows app theme without a polling thread.
pub fn prefers_dark() -> bool {
    let mut value = 1u32;
    let mut bytes = 4u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut bytes),
        ) == ERROR_SUCCESS
            && value == 0
    }
}

/// A native dialog runs on a short-lived STA thread. The caller serializes
/// dialogs and pauses the timer before opening one.
pub fn choose_file(kind: u32, owner: Option<isize>, callback: EventHandler) {
    let handler = callback.clone();
    if let Err(error) = thread::Builder::new()
        .name("orbit-file-dialog".into())
        .spawn(move || {
            let result = unsafe {
                (|| -> Result<Option<PathBuf>> {
                    CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                        .ok()
                        .map_err(|e| e.to_string())?;
                    let result = (|| -> windows::core::Result<Option<PathBuf>> {
                        let class = if kind == 1 {
                            &FileOpenDialog
                        } else {
                            &FileSaveDialog
                        };
                        let dialog: IFileDialog =
                            CoCreateInstance(class, None, CLSCTX_INPROC_SERVER)?;
                        dialog.SetOptions(
                            dialog.GetOptions()?
                                | FOS_FORCEFILESYSTEM
                                | if kind == 1 {
                                    FOS_FILEMUSTEXIST
                                } else {
                                    FOS_PATHMUSTEXIST
                                },
                        )?;
                        dialog.SetTitle(&HSTRING::from(if kind == 1 {
                            "Restore an Orbit backup"
                        } else if kind == 2 {
                            "Export focus report"
                        } else {
                            "Create an Orbit backup"
                        }))?;
                        if kind != 1 {
                            dialog.SetDefaultExtension(if kind == 2 {
                                w!("csv")
                            } else {
                                w!("db")
                            })?;
                            dialog.SetFileName(&HSTRING::from(format!(
                                "orbit-{}.{}",
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                if kind == 2 { "csv" } else { "db" }
                            )))?;
                        }
                        if let Err(error) = dialog.Show(owner.map(|h| HWND(h as *mut c_void))) {
                            if error.code().0 as u32 == 0x800704c7 {
                                return Ok(None);
                            }
                            return Err(error);
                        }
                        let value = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
                        let path = value.to_string();
                        CoTaskMemFree(Some(value.0.cast()));
                        Ok(Some(PathBuf::from(path?)))
                    })()
                    .map_err(|e| e.to_string());
                    CoUninitialize();
                    result
                })()
            };
            callback(Event::FileDialog { kind, result });
        })
    {
        handler(Event::FileDialog {
            kind,
            result: Err(error.to_string()),
        });
    }
}
enum NativeCommand {
    Hotkeys([String; 3], mpsc::SyncSender<Vec<String>>),
    Tooltip(String),
    Balloon(String, String),
}
#[derive(Clone)]
pub struct Platform {
    inner: Arc<PlatformInner>,
}
struct PlatformInner {
    hwnd: isize,
    sender: mpsc::SyncSender<NativeCommand>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    notifying: Arc<AtomicBool>,
}
impl PlatformInner {
    fn stop(&self) {
        let mut thread = self.thread.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(thread) = thread.take() {
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(self.hwnd as *mut c_void)),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
            let _ = thread.join();
        }
    }
}
impl Drop for PlatformInner {
    fn drop(&mut self) {
        self.stop();
    }
}
struct NativeState {
    callback: EventHandler,
    receiver: mpsc::Receiver<NativeCommand>,
    icon: HICON,
    taskbar_created: u32,
    hotkeys: [String; 3],
}
thread_local! { static NATIVE: RefCell<Option<NativeState>> = const { RefCell::new(None) }; }

pub struct Instance {
    handle: HANDLE,
}
impl Instance {
    /// A second launch activates the existing instance rather than opening its database.
    pub fn acquire(name: &str) -> Result<Option<Self>> {
        let name = HSTRING::from(name);
        let handle = unsafe { CreateMutexW(None, false, &name) }.map_err(|e| e.to_string())?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                let _ = CloseHandle(handle);
                if let Ok(window) = FindWindowW(WINDOW_CLASS, None) {
                    let _ = PostMessageW(Some(window), WM_ACTIVATE_ORBIT, WPARAM(0), LPARAM(0));
                }
            }
            Ok(None)
        } else {
            Ok(Some(Self { handle }))
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Platform {
    pub fn start(callback: EventHandler) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(64);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("orbit-shell".into())
            .spawn(move || unsafe {
                let module = match GetModuleHandleW(None) {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(window_proc),
                    hInstance: module.into(),
                    lpszClassName: WINDOW_CLASS,
                    ..Default::default()
                };
                RegisterClassW(&wc);
                let hwnd = match CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    WINDOW_CLASS,
                    w!("Orbit integration"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Some(module.into()),
                    None,
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let icon = create_tray_icon();
                NATIVE.with_borrow_mut(|s| {
                    *s = Some(NativeState {
                        callback,
                        receiver,
                        icon,
                        taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
                        hotkeys: Default::default(),
                    })
                });
                add_tray(hwnd, icon);
                let _ = ready_tx.send(Ok(hwnd.0 as isize));
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                NATIVE.with_borrow_mut(|s| {
                    if let Some(s) = s.take() {
                        let data = tray_data(hwnd, s.icon);
                        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
                        let _ = DestroyIcon(s.icon);
                    }
                });
                let _ = UnregisterClassW(WINDOW_CLASS, Some(module.into()));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: Arc::new(PlatformInner {
                hwnd: ready_rx.recv().map_err(|e| e.to_string())??,
                sender,
                thread: Mutex::new(Some(thread)),
                notifying: Arc::new(AtomicBool::new(false)),
            }),
        })
    }
    fn send(&self, command: NativeCommand) -> Result<()> {
        self.inner
            .sender
            .try_send(command)
            .map_err(|e| e.to_string())?;
        unsafe {
            PostMessageW(
                Some(HWND(self.inner.hwnd as *mut c_void)),
                WM_COMMANDS,
                WPARAM(0),
                LPARAM(0),
            )
        }
        .map_err(|e| e.to_string())
    }
    pub fn hotkeys(&self, settings: &Settings) -> Vec<String> {
        let values = [
            settings.hotkey_toggle.clone(),
            settings.hotkey_show.clone(),
            settings.hotkey_skip.clone(),
        ];
        let (tx, rx) = mpsc::sync_channel(1);
        if let Err(e) = self.send(NativeCommand::Hotkeys(values, tx)) {
            return vec![e];
        }
        rx.recv().unwrap_or_else(|e| vec![e.to_string()])
    }
    pub fn tooltip(&self, text: &str) {
        let _ = self.send(NativeCommand::Tooltip(text.into()));
    }
    pub fn balloon(&self, title: &str, body: &str) {
        let _ = self.send(NativeCommand::Balloon(title.into(), body.into()));
    }
    pub fn stop(&self) {
        self.inner.stop();
    }
    pub fn notify(&self, title: &str, body: &str) {
        let Some(guard) = BusyGuard::acquire(self.inner.notifying.clone()) else {
            return;
        };
        let title = title.to_string();
        let body = body.to_string();
        let platform = self.clone();
        let _ = thread::Builder::new()
            .name("orbit-toast".into())
            .spawn(move || {
                let _guard = guard;
                let result = show_toast(&title, &body);
                if result.is_err() {
                    platform.balloon(&title, &body);
                }
            });
    }
}
fn emit(event: Event) {
    NATIVE.with_borrow(|s| {
        if let Some(s) = s {
            (s.callback)(event);
        }
    });
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let restored = NATIVE.with_borrow(|s| s.as_ref().is_some_and(|s| msg == s.taskbar_created));
        if restored {
            NATIVE.with_borrow(|s| {
                if let Some(s) = s {
                    add_tray(hwnd, s.icon);
                }
            });
            return LRESULT(0);
        }
        match msg {
            WM_TRAY => {
                match lparam.0 as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK | NIN_BALLOONUSERCLICK => emit(Event::Show),
                    WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(hwnd),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                menu_command(wparam.0 as u32 & 0xffff);
                LRESULT(0)
            }
            WM_HOTKEY => {
                match wparam.0 {
                    1 => emit(Event::Toggle),
                    2 => emit(Event::Show),
                    3 => emit(Event::Skip),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_ACTIVATE_ORBIT => {
                emit(Event::Show);
                LRESULT(0)
            }
            WM_POWERBROADCAST => {
                match wparam.0 as u32 {
                    4 => emit(Event::Suspend),
                    7 | 18 => emit(Event::Resume),
                    _ => {}
                }
                LRESULT(1)
            }
            WM_DISPLAYCHANGE => {
                emit(Event::DisplayChanged);
                LRESULT(0)
            }
            WM_SETTINGCHANGE | WM_THEMECHANGED => {
                emit(Event::AppearanceChanged);
                LRESULT(0)
            }
            WM_QUERYENDSESSION => {
                emit(Event::Suspend);
                LRESULT(1)
            }
            WM_ENDSESSION if wparam.0 != 0 => {
                emit(Event::Shutdown);
                LRESULT(0)
            }
            WM_ENDSESSION => {
                emit(Event::Resume);
                LRESULT(0)
            }
            WM_COMMANDS => {
                // Drain commands without holding a RefCell borrow across Win32 message dispatch.
                loop {
                    let command =
                        NATIVE.with_borrow(|s| s.as_ref().and_then(|s| s.receiver.try_recv().ok()));
                    match command {
                        Some(NativeCommand::Hotkeys(keys, reply)) => {
                            let warnings = register_hotkeys(hwnd, &keys);
                            NATIVE.with_borrow_mut(|s| {
                                if let Some(s) = s {
                                    s.hotkeys = keys;
                                }
                            });
                            let _ = reply.send(warnings);
                        }
                        Some(NativeCommand::Tooltip(text)) => {
                            NATIVE.with_borrow(|s| {
                                if let Some(s) = s {
                                    let mut data = tray_data(hwnd, s.icon);
                                    data.uFlags = NIF_TIP;
                                    wide_into(&text, &mut data.szTip);
                                    let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
                                }
                            });
                        }
                        Some(NativeCommand::Balloon(title, body)) => {
                            NATIVE.with_borrow(|s| {
                                if let Some(s) = s {
                                    let mut data = tray_data(hwnd, s.icon);
                                    data.uFlags = NIF_INFO;
                                    wide_into(&title, &mut data.szInfoTitle);
                                    wide_into(&body, &mut data.szInfo);
                                    data.dwInfoFlags = NIIF_INFO;
                                    let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
                                }
                            });
                        }
                        None => break,
                    }
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
unsafe fn show_menu(hwnd: HWND) {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        for (id, label) in [
            (1, "Open Orbit"),
            (2, "Start / pause"),
            (3, "Skip session"),
            (4, "Reset session"),
            (0, ""),
            (10, "Toggle timer ring"),
            (11, "Toggle today's tasks"),
            (12, "Toggle mini timer"),
            (20, "Unlock all widgets"),
            (21, "Reset widget positions"),
            (0, ""),
            (5, "Quit Orbit"),
        ] {
            let label = HSTRING::from(label);
            let _ = AppendMenuW(
                menu,
                if id == 0 { MF_SEPARATOR } else { MF_STRING },
                id,
                &label,
            );
        }
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let _ = SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            cursor.x,
            cursor.y,
            Some(0),
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        if chosen.0 != 0 {
            menu_command(chosen.0 as u32);
        }
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    }
}
fn menu_command(id: u32) {
    match id {
        1 => emit(Event::Show),
        2 => emit(Event::Toggle),
        3 => emit(Event::Skip),
        4 => emit(Event::Reset),
        5 => emit(Event::Quit),
        10..=12 => emit(Event::ToggleWidget((id - 10) as usize)),
        20 => emit(Event::UnlockWidgets),
        21 => emit(Event::ResetWidgets),
        _ => {}
    }
}
fn wide_into(text: &str, output: &mut [u16]) {
    let count = output.len().saturating_sub(1);
    output.fill(0);
    for (a, b) in output.iter_mut().take(count).zip(text.encode_utf16()) {
        *a = b;
    }
}
fn tray_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: icon,
        ..Default::default()
    }
}
unsafe fn add_tray(hwnd: HWND, icon: HICON) {
    unsafe {
        let mut data = tray_data(hwnd, icon);
        wide_into("Orbit · Focus timer", &mut data.szTip);
        if !Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
            emit(Event::Error(
                "Could not create the tray icon. Keep the main window open.".into(),
            ));
        }
    }
}
unsafe fn create_tray_icon() -> HICON {
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let d = ((x as f32 - 15.5).powi(2) + (y as f32 - 15.5).powi(2)).sqrt();
            let i = (y * 32 + x) * 4;
            if (9.0..14.0).contains(&d) || ((14..=17).contains(&x) && (9..=17).contains(&y)) {
                pixels[i] = 53;
                pixels[i + 1] = 71;
                pixels[i + 2] = 183;
                pixels[i + 3] = 255;
            }
        }
    }
    unsafe {
        CreateIcon(
            None,
            32,
            32,
            1,
            32,
            vec![0u8; 128].as_ptr(),
            pixels.as_ptr(),
        )
        .unwrap_or_default()
    }
}

pub fn parse_hotkey(text: &str) -> Result<Option<(HOT_KEY_MODIFIERS, u32)>> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let mut modifiers = MOD_NOREPEAT;
    let mut key = None;
    for token in text.split('+').map(|s| s.trim().to_ascii_uppercase()) {
        match token.as_str() {
            "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL,
            "ALT" => modifiers |= MOD_ALT,
            "SHIFT" => modifiers |= MOD_SHIFT,
            "WIN" => modifiers |= MOD_WIN,
            _ => {
                if key.is_some() {
                    return Err(format!("Invalid shortcut: {text}"));
                }
                key = Some(if token == "SPACE" {
                    0x20
                } else if token.len() == 1 && token.as_bytes()[0].is_ascii_alphanumeric() {
                    u32::from(token.as_bytes()[0])
                } else if let Some(n) = token
                    .strip_prefix('F')
                    .and_then(|x| x.parse::<u32>().ok())
                    .filter(|n| (1..=12).contains(n))
                {
                    0x6f + n
                } else {
                    return Err(format!("Invalid shortcut key: {text}"));
                });
            }
        }
    }
    let key = key.ok_or_else(|| format!("Shortcut needs a key: {text}"))?;
    if modifiers == MOD_NOREPEAT {
        return Err("Global shortcuts need Ctrl, Alt, Shift, or Win.".into());
    }
    Ok(Some((modifiers, key)))
}
pub fn validate_hotkeys(settings: &Settings) -> Result<()> {
    let parsed = [
        parse_hotkey(&settings.hotkey_toggle)?,
        parse_hotkey(&settings.hotkey_show)?,
        parse_hotkey(&settings.hotkey_skip)?,
    ];
    for i in 0..3 {
        for j in i + 1..3 {
            if parsed[i].is_some() && parsed[i] == parsed[j] {
                return Err("Each enabled shortcut must be different.".into());
            }
        }
    }
    Ok(())
}
unsafe fn register_hotkeys(hwnd: HWND, keys: &[String; 3]) -> Vec<String> {
    let mut warnings = Vec::new();
    for i in 1..=3 {
        unsafe {
            let _ = UnregisterHotKey(Some(hwnd), i);
        }
    }
    for (i, key) in keys.iter().enumerate() {
        match parse_hotkey(key) {
            Ok(Some((modifiers, vk))) => {
                if unsafe { RegisterHotKey(Some(hwnd), i as i32 + 1, modifiers, vk) }.is_err() {
                    warnings.push(format!("{key} is already in use. Choose another shortcut."));
                }
            }
            Err(e) => warnings.push(e),
            _ => {}
        }
    }
    warnings
}

pub fn data_directory() -> PathBuf {
    data_directory_in(
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir),
    )
}
fn data_directory_in(base: PathBuf) -> PathBuf {
    let orbit = base.join("Orbit");
    let legacy = base.join("StillPomodoro");
    if !orbit.join("orbit.db").is_file() && legacy.join("still.db").is_file() {
        legacy
    } else {
        orbit
    }
}
pub fn database_path(directory: &Path) -> PathBuf {
    let orbit = directory.join("orbit.db");
    let legacy = directory.join("still.db");
    if !orbit.is_file() && legacy.is_file() {
        legacy
    } else {
        orbit
    }
}
pub fn message_box(title: &str, message: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(message),
            &HSTRING::from(title),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub fn open_folder(path: &Path) {
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(path.as_os_str()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}
pub fn set_autostart(enabled: bool) -> Result<()> {
    let exe = if enabled {
        Some(std::env::current_exe().map_err(|e| e.to_string())?)
    } else {
        None
    };
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()
        .map_err(|e| e.to_string())?;
        let result = if let Some(exe) = exe {
            let value: Vec<u16> = format!("\"{}\" --tray", exe.display())
                .encode_utf16()
                .chain(Some(0))
                .collect();
            RegSetValueExW(
                key,
                w!("Orbit"),
                Some(0),
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    value.as_ptr() as *const u8,
                    value.len() * 2,
                )),
            )
            .ok()
        } else {
            let status = RegDeleteValueW(key, w!("Orbit"));
            if status == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                status.ok()
            }
        };
        // Remove the previous name only after the replacement was successfully saved.
        if result.is_ok() {
            let _ = RegDeleteValueW(key, w!("StillPomodoro"));
        }
        let _ = RegCloseKey(key);
        result.map_err(|e| e.to_string())
    }
}

// Subclass only observes lifecycle and pointer hit-testing; Winit retains ownership of its WndProc.
struct Hook {
    index: Option<usize>,
    callback: EventHandler,
    click_through: bool,
    layer: i32,
}
thread_local! {static HOOKS:RefCell<HashMap<isize,Hook>>=RefCell::new(HashMap::new());}
pub fn attach_window(hwnd: isize, index: Option<usize>, callback: EventHandler) {
    // Do not retain a callback for a window if installing its destruction hook failed.
    if !unsafe { SetWindowSubclass(HWND(hwnd as *mut c_void), Some(window_subclass), 77, 0) }
        .as_bool()
    {
        return;
    }
    HOOKS.with_borrow_mut(|h| {
        h.insert(
            hwnd,
            Hook {
                index,
                callback,
                click_through: false,
                layer: 1,
            },
        );
    });
}
pub fn detach_window(hwnd: isize) {
    unsafe {
        let _ = RemoveWindowSubclass(HWND(hwnd as *mut c_void), Some(window_subclass), 77);
    }
    HOOKS.with_borrow_mut(|h| {
        h.remove(&hwnd);
    });
}
unsafe extern "system" fn window_subclass(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    unsafe {
        let info = HOOKS.with_borrow(|h| {
            h.get(&(hwnd.0 as isize))
                .map(|h| (h.index, h.callback.clone(), h.click_through, h.layer))
        });
        if let Some((index, callback, click_through, layer)) = info {
            match msg {
                WM_NCHITTEST if click_through => return LRESULT(HTTRANSPARENT as isize),
                WM_WINDOWPOSCHANGING if layer == 0 && l.0 != 0 => {
                    let position = &mut *(l.0 as *mut WINDOWPOS);
                    position.hwndInsertAfter = HWND_BOTTOM;
                    position.flags &= !SWP_NOZORDER;
                }
                WM_EXITSIZEMOVE => {
                    if let Some(index) = index {
                        let mut rect = RECT::default();
                        if GetWindowRect(hwnd, &mut rect).is_ok() {
                            callback(Event::WidgetMoved {
                                index,
                                x: rect.left,
                                y: rect.top,
                            });
                        }
                    }
                    if layer == 0 {
                        let _ = SetWindowPos(
                            hwnd,
                            Some(HWND_BOTTOM),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                WM_DISPLAYCHANGE | WM_DPICHANGED => callback(Event::DisplayChanged),
                WM_SIZE | WM_SHOWWINDOW => callback(Event::VisibilityChanged),
                WM_NCDESTROY => {
                    HOOKS.with_borrow_mut(|h| {
                        h.remove(&(hwnd.0 as isize));
                    });
                }
                _ => {}
            }
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
pub fn apply_widget(hwnd: isize, preferences: &WidgetPreferences) {
    let hwnd = HWND(hwnd as *mut c_void);
    HOOKS.with_borrow_mut(|h| {
        if let Some(h) = h.get_mut(&(hwnd.0 as isize)) {
            h.click_through = preferences.click_through;
            h.layer = preferences.layer;
        }
    });
    unsafe {
        let mut style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        style |= WS_EX_TOOLWINDOW.0;
        style &= !WS_EX_APPWINDOW.0;
        if preferences.click_through {
            style |= WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0;
        } else {
            style &= !(WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0);
        }
        SetWindowLongW(hwnd, GWL_EXSTYLE, style as i32);
        if preferences.click_through {
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
        }
        let after = match preferences.layer {
            0 => HWND_BOTTOM,
            2 => HWND_TOPMOST,
            _ => HWND_NOTOPMOST,
        };
        let _ = SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        rounded(hwnd);
    }
}
pub fn configure_main(hwnd: isize, mica: bool) -> bool {
    let hwnd = HWND(hwnd as *mut c_void);
    unsafe {
        rounded(hwnd);
        let backdrop = if mica { DWMSBT_MAINWINDOW } else { DWMSBT_NONE };
        let result = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const c_void,
            size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
        let margin = if mica { -1 } else { 0 };
        let margins = windows::Win32::UI::Controls::MARGINS {
            cxLeftWidth: margin,
            cxRightWidth: margin,
            cyTopHeight: margin,
            cyBottomHeight: margin,
        };
        let extended = DwmExtendFrameIntoClientArea(hwnd, &margins);
        mica && result.is_ok() && extended.is_ok()
    }
}
unsafe fn rounded(hwnd: HWND) {
    unsafe {
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const c_void,
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}
pub fn foreground(hwnd: isize) {
    unsafe {
        let hwnd = HWND(hwnd as *mut c_void);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}
pub fn clamp_position(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return (x.max(0), y.max(0));
        }
        let r = info.rcWork;
        (
            x.clamp(r.left, (r.right - width).max(r.left)),
            y.clamp(r.top, (r.bottom - height).max(r.top)),
        )
    }
}
pub fn high_contrast() -> bool {
    unsafe {
        let mut value = HIGHCONTRASTW {
            cbSize: size_of::<HIGHCONTRASTW>() as u32,
            ..Default::default()
        };
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            value.cbSize,
            Some(&mut value as *mut _ as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && (value.dwFlags.0 & 1) != 0
    }
}
pub fn reduced_motion() -> bool {
    let mut enabled = 1i32;
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut i32).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && enabled == 0
    }
}
struct BusyGuard(Arc<AtomicBool>);
impl BusyGuard {
    fn acquire(busy: Arc<AtomicBool>) -> Option<Self> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(busy))
    }
}
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub fn play_alarm(sound: u32, volume: u32, callback: EventHandler) {
    static BUSY: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();
    if volume == 0 {
        return;
    }
    let Some(guard) = BusyGuard::acquire(
        BUSY.get_or_init(|| Arc::new(AtomicBool::new(false)))
            .clone(),
    ) else {
        return;
    };
    let _ = thread::Builder::new()
        .name("orbit-alarm".into())
        .spawn(move || {
            let _guard = guard;
            if let Err(e) = play_pcm(sound, volume) {
                callback(Event::Error(e));
            }
        });
}
fn alarm_wave(sound: u32, volume: u32) -> Vec<u8> {
    const RATE: u32 = 22050;
    let samples = if sound == 2 { 24255 } else { 15435 };
    let data_bytes = samples * 2;
    let mut wave = Vec::with_capacity(44 + data_bytes);
    wave.extend_from_slice(b"RIFF");
    wave.extend_from_slice(&(36 + data_bytes as u32).to_le_bytes());
    wave.extend_from_slice(b"WAVEfmt ");
    wave.extend_from_slice(&16u32.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wave.extend_from_slice(&1u16.to_le_bytes()); // mono
    wave.extend_from_slice(&RATE.to_le_bytes());
    wave.extend_from_slice(&(RATE * 2).to_le_bytes());
    wave.extend_from_slice(&2u16.to_le_bytes());
    wave.extend_from_slice(&16u16.to_le_bytes());
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for i in 0..samples {
        let time = i as f32 / RATE as f32;
        let offset = if sound == 2 && time > 0.5 {
            time - 0.5
        } else {
            time
        };
        let frequency = if sound == 1 { 660.0 } else { 880.0 };
        let envelope = (offset * 120.0).min(1.0) * (-offset * 7.0).exp();
        let sample = ((std::f32::consts::TAU * frequency * offset).sin()
            + 0.22 * (std::f32::consts::TAU * frequency * 2.0 * offset).sin())
            * envelope;
        let sample = (sample * 10000.0 * volume.min(100) as f32 / 100.0) as i16;
        wave.extend_from_slice(&sample.to_le_bytes());
    }
    wave
}
fn play_pcm(sound: u32, volume: u32) -> Result<()> {
    let wave = alarm_wave(sound, volume);
    // Synchronous playback on the bounded alarm thread keeps this complete RIFF buffer
    // alive until Windows releases it. No application-owned WAVEHDR or driver buffer.
    if unsafe {
        PlaySoundW(
            PCWSTR(wave.as_ptr().cast()),
            None,
            SND_MEMORY | SND_SYNC | SND_NODEFAULT,
        )
    }
    .as_bool()
    {
        Ok(())
    } else {
        Err("No audio output is available, or the alarm could not be played.".into())
    }
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn show_toast(title: &str, body: &str) -> Result<()> {
    if !notification_registration_matches() {
        return Err("Notifications are not registered for this executable.".into());
    }
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| e.to_string())?;
    }
    let result = (|| {
        let xml = XmlDocument::new().map_err(|e| e.to_string())?;
        xml.LoadXml(&HSTRING::from(format!("<toast activationType=\"protocol\" launch=\"orbit-pomodoro:open\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><audio silent=\"true\"/></toast>",escape_xml(title),escape_xml(body)))).map_err(|e|e.to_string())?;
        let toast = ToastNotification::CreateToastNotification(&xml).map_err(|e| e.to_string())?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))
            .and_then(|n| n.Show(&toast))
            .map_err(|e| e.to_string())
    })();
    unsafe {
        CoUninitialize();
    }
    result
}

fn notification_registration_matches() -> bool {
    let Some(roaming) = std::env::var_os("APPDATA") else {
        return false;
    };
    if !PathBuf::from(roaming)
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Orbit.lnk")
        .is_file()
    {
        return false;
    }
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    let expected = format!("\"{}\" --show", executable.display());
    let mut value = [0u16; 32768];
    let mut bytes = (value.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Classes\\orbit-pomodoro\\shell\\open\\command"),
            None,
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr() as *mut c_void),
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return false;
    }
    let length = value.iter().position(|ch| *ch == 0).unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length]).eq_ignore_ascii_case(&expected)
}

/// Explicit portable setup. A stub activator and protocol avoid a Windows App SDK dependency.
pub fn register_notifications() -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let roaming = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or("APPDATA is unavailable")?;
    let shortcut = roaming.join("Microsoft\\Windows\\Start Menu\\Programs\\Orbit.lnk");
    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let uninitialize = init.is_ok();
        if init.is_err() && init.0 != RPC_E_CHANGED_MODE.0 {
            return Err("Could not initialize notification registration.".into());
        }
        let result = (|| -> Result<()> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| e.to_string())?;
            link.SetPath(&HSTRING::from(exe.as_os_str()))
                .map_err(|e| e.to_string())?;
            link.SetDescription(w!("Orbit focus timer"))
                .map_err(|e| e.to_string())?;
            let properties: IPropertyStore = link.cast().map_err(|e| e.to_string())?;
            // PROPVARIANT string is owned by this HSTRING until SetValue returns.
            let mut app_id: Vec<u16> = APP_ID.encode_utf16().chain(Some(0)).collect();
            let value = windows::Win32::System::Com::StructuredStorage::PROPVARIANT {
                Anonymous: windows::Win32::System::Com::StructuredStorage::PROPVARIANT_0 {
                    Anonymous: std::mem::ManuallyDrop::new(
                        windows::Win32::System::Com::StructuredStorage::PROPVARIANT_0_0 {
                            vt: windows::Win32::System::Variant::VT_LPWSTR,
                            Anonymous:
                                windows::Win32::System::Com::StructuredStorage::PROPVARIANT_0_0_0 {
                                    pwszVal: PWSTR(app_id.as_mut_ptr()),
                                },
                            ..Default::default()
                        },
                    ),
                },
            };
            properties
                .SetValue(&PKEY_AppUserModel_ID, &value)
                .map_err(|e| e.to_string())?;
            // Protocol-only activation uses a stub CLSID so Action Center retains the toast.
            let mut clsid = windows::core::GUID::from_u128(0x1b59d91c_8435_40d2_8a63_a582210ca911);
            let activator = windows::Win32::System::Com::StructuredStorage::PROPVARIANT {
                Anonymous: windows::Win32::System::Com::StructuredStorage::PROPVARIANT_0 {
                    Anonymous: std::mem::ManuallyDrop::new(
                        windows::Win32::System::Com::StructuredStorage::PROPVARIANT_0_0 {
                            vt: windows::Win32::System::Variant::VT_CLSID,
                            Anonymous:
                                windows::Win32::System::Com::StructuredStorage::PROPVARIANT_0_0_0 {
                                    puuid: &mut clsid,
                                },
                            ..Default::default()
                        },
                    ),
                },
            };
            properties
                .SetValue(&PKEY_AppUserModel_ToastActivatorCLSID, &activator)
                .map_err(|e| e.to_string())?;
            properties.Commit().map_err(|e| e.to_string())?;
            let persist: IPersistFile = link.cast().map_err(|e| e.to_string())?;
            persist
                .Save(&HSTRING::from(shortcut.as_os_str()), true)
                .map_err(|e| e.to_string())?;
            set_registry_string("Software\\Classes\\orbit-pomodoro", None, "URL:Orbit")?;
            set_registry_string(
                "Software\\Classes\\orbit-pomodoro",
                Some("URL Protocol"),
                "",
            )?;
            set_registry_string(
                "Software\\Classes\\orbit-pomodoro\\shell\\open\\command",
                None,
                &format!("\"{}\" --show", exe.display()),
            )?;
            Ok(())
        })();
        if uninitialize {
            CoUninitialize();
        }
        result
    }
}
fn set_registry_string(path: &str, name: Option<&str>, value: &str) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(path),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()
        .map_err(|e| e.to_string())?;
        let value: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        let name = name.map(HSTRING::from);
        let result = RegSetValueExW(
            key,
            name.as_ref().map_or(PCWSTR::null(), |s| PCWSTR(s.as_ptr())),
            Some(0),
            REG_SZ,
            Some(std::slice::from_raw_parts(
                value.as_ptr() as *const u8,
                value.len() * 2,
            )),
        )
        .ok()
        .map_err(|e| e.to_string());
        let _ = RegCloseKey(key);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canceled_shutdown_resumes_the_timer() {
        let (sender, receiver) = mpsc::channel();
        let (_commands, commands) = mpsc::sync_channel(1);
        NATIVE.with_borrow_mut(|slot| {
            *slot = Some(NativeState {
                callback: Arc::new(move |event| sender.send(event).unwrap()),
                receiver: commands,
                icon: HICON::default(),
                taskbar_created: u32::MAX,
                hotkeys: Default::default(),
            })
        });
        unsafe {
            window_proc(HWND::default(), WM_QUERYENDSESSION, WPARAM(0), LPARAM(0));
            window_proc(HWND::default(), WM_ENDSESSION, WPARAM(0), LPARAM(0));
        }
        NATIVE.with_borrow_mut(|slot| *slot = None);
        let events: Vec<_> = receiver.try_iter().collect();
        assert!(matches!(events.as_slice(), [Event::Suspend, Event::Resume]));
        let mut timer = focus_core::Timer::new(Settings::default());
        let now = WindowsClock.now();
        timer.dispatch(focus_core::Command::Start, now);
        timer.dispatch(focus_core::Command::Suspend, now);
        timer.dispatch(focus_core::Command::Resume, now);
        timer.dispatch(focus_core::Command::Pause, now);
        timer.dispatch(focus_core::Command::Start, now);
        assert!(timer.snapshot(now).running);
    }
    #[test]
    fn orbit_reuses_existing_databases_without_moving_or_overwriting_them() {
        let base = std::env::temp_dir().join(format!(
            "orbit-rename-{}-{}",
            std::process::id(),
            WindowsClock.now().awake_ms
        ));
        let legacy = base.join("StillPomodoro");
        let orbit = base.join("Orbit");
        assert_eq!(data_directory_in(base.clone()), orbit);
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("still.db"), b"saved history").unwrap();
        assert_eq!(data_directory_in(base.clone()), legacy);
        assert_eq!(database_path(&legacy), legacy.join("still.db"));
        std::fs::create_dir_all(&orbit).unwrap();
        std::fs::write(orbit.join("orbit.db"), b"new history").unwrap();
        assert_eq!(data_directory_in(base), orbit);
        assert_eq!(database_path(&orbit), orbit.join("orbit.db"));
        assert_eq!(
            std::fs::read(legacy.join("still.db")).unwrap(),
            b"saved history"
        );
        std::fs::remove_file(legacy.join("still.db")).unwrap();
        std::fs::remove_file(orbit.join("orbit.db")).unwrap();
        std::fs::remove_dir(legacy).unwrap();
        std::fs::remove_dir(&orbit).unwrap();
        std::fs::remove_dir(orbit.parent().unwrap()).unwrap();
    }
    fn handle_count() -> u32 {
        let mut count = 0;
        unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) }.unwrap();
        count
    }
    #[test]
    #[ignore = "Needs an interactive Windows desktop; creates only hidden helper windows"]
    fn shell_cycles_release_threads_icons_handles_and_callbacks() {
        let callback: EventHandler = Arc::new(|_| {});
        drop(Platform::start(callback.clone()).unwrap());
        let before = handle_count();
        let gdi = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        let user = unsafe { GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS) };
        for _ in 0..200 {
            let platform = Platform::start(callback.clone()).unwrap();
            for _ in 0..100 {
                platform.tooltip("Still stress test");
            }
            drop(platform);
            assert_eq!(Arc::strong_count(&callback), 1);
        }
        assert!(
            handle_count() <= before + 2,
            "handles: {before} -> {}",
            handle_count()
        );
        assert_eq!(
            unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) },
            gdi
        );
        assert_eq!(
            unsafe { GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS) },
            user
        );
    }
    #[test]
    #[ignore = "Exercises the real Windows audio driver with silent samples"]
    fn silent_native_playback_releases_resources() {
        let _ = play_pcm(0, 0); // warm system audio libraries before comparing handles
        let before = handle_count();
        let mut unavailable = 0;
        for _ in 0..20 {
            if play_pcm(0, 0).is_err() {
                unavailable += 1;
            }
        }
        thread::sleep(std::time::Duration::from_millis(500));
        assert!(
            handle_count() <= before + 2,
            "handles: {before} -> {}",
            handle_count()
        );
        eprintln!("Silent native playback: {unavailable}/20 calls reported unavailable audio");
    }
    #[test]
    fn playback_gate_bounds_concurrent_resources_and_releases_on_error() {
        let busy = Arc::new(AtomicBool::new(false));
        let guard = BusyGuard::acquire(busy.clone()).unwrap();
        for _ in 0..100_000 {
            assert!(BusyGuard::acquire(busy.clone()).is_none());
        }
        drop(guard);
        assert!(BusyGuard::acquire(busy.clone()).is_some());
        assert!(!busy.load(Ordering::Acquire));
    }
    #[test]
    fn alarms_have_valid_pcm_lengths_and_zero_volume_is_silent() {
        for sound in 0..3 {
            let wave = alarm_wave(sound, 0);
            assert_eq!(&wave[..4], b"RIFF");
            assert_eq!(&wave[8..16], b"WAVEfmt ");
            assert_eq!(
                u32::from_le_bytes(wave[4..8].try_into().unwrap()) as usize + 8,
                wave.len()
            );
            assert_eq!(
                u32::from_le_bytes(wave[40..44].try_into().unwrap()) as usize + 44,
                wave.len()
            );
            assert!(wave[44..].iter().all(|byte| *byte == 0));
            assert_eq!(alarm_wave(sound, 100), alarm_wave(sound, 200));
        }
    }
    #[test]
    fn parses_shortcuts_and_rejects_unsafe_or_duplicate_bindings() {
        assert_eq!(parse_hotkey("Ctrl+Alt+P").unwrap().unwrap().1, 0x50);
        assert_eq!(parse_hotkey("Shift+F12").unwrap().unwrap().1, 0x7b);
        assert!(parse_hotkey("P").is_err());
        assert!(parse_hotkey("Ctrl+P+Q").is_err());
        assert!(parse_hotkey("").unwrap().is_none());
        let mut s = Settings::default();
        s.hotkey_skip = s.hotkey_toggle.clone();
        assert!(validate_hotkeys(&s).is_err());
    }
    #[test]
    fn awake_clock_is_monotonic() {
        let a = WindowsClock.now();
        let b = WindowsClock.now();
        assert!(b.awake_ms >= a.awake_ms);
    }
}
