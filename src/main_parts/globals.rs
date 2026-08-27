static STATE: LazyLock<Mutex<AppState>> = LazyLock::new(|| Mutex::new(AppState::default()));
static SETTINGS_HOTKEY_RECORDING: AtomicBool = AtomicBool::new(false);
static MOUSE_HOTKEYS_ENABLED: AtomicBool = AtomicBool::new(true);
static TRAY_ICON_ADDED: AtomicBool = AtomicBool::new(false);
static SETTINGS_MICA_ENABLED: AtomicBool = AtomicBool::new(false);
static TASKBAR_CREATED_MESSAGE: LazyLock<u32> =
    LazyLock::new(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });

thread_local! {
    static AUDIO_NOTIFICATION_REGISTRATION: RefCell<Option<AudioNotificationRegistration>> =
        const { RefCell::new(None) };
}

static SETTINGS_MOUSE_HELD: LazyLock<Mutex<Vec<u32>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static SETTINGS_MOUSE_CHORD: LazyLock<Mutex<Vec<u32>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static SETTINGS_KEYBOARD_HELD: LazyLock<Mutex<Vec<u32>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static SETTINGS_KEYBOARD_CHORD: LazyLock<Mutex<Vec<u32>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static SETTINGS_CAPTURE_LAST_EVENT: LazyLock<Mutex<Option<Instant>>> =
    LazyLock::new(|| Mutex::new(None));
static SETTINGS_PRESSED_SHORTCUT: LazyLock<Mutex<Option<Shortcut>>> =
    LazyLock::new(|| Mutex::new(None));
static PENDING_NOTIFICATION_ACTION: LazyLock<Mutex<Option<NotificationAction>>> =
    LazyLock::new(|| Mutex::new(None));
static SETTINGS_ORIGINAL_WNDPROC: AtomicIsize = AtomicIsize::new(0);
static NEXT_HOTKEY_ID: AtomicU64 = AtomicU64::new(1);
#[cfg(target_pointer_width = "32")]
type WindowLongPtrValue = i32;
#[cfg(target_pointer_width = "64")]
type WindowLongPtrValue = isize;
