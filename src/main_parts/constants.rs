const WM_TRAY: u32 = WM_APP + 1;
const WM_TOGGLE_MUTE: u32 = WM_APP + 2;
const WM_OPEN_SETTINGS: u32 = WM_APP + 4;
const WM_EXIT_ALL: u32 = WM_APP + 5;
const WM_AUDIO_MUTE_STATE_CHANGED: u32 = WM_APP + 6;
const WM_AUDIO_ENDPOINT_CHANGED: u32 = WM_APP + 7;
const WM_CONFIG_CHANGED: u32 = WM_APP + 8;
const WM_PROCESS_HOTKEY_ACTIONS: u32 = WM_APP + 9;
const WM_PREVIEW_OVERLAY: u32 = WM_APP + 10;
const WM_DEFAULT_CAPTURE_DEVICE_CHANGED: u32 = WM_APP + 11;
const WM_PREVIEW_SOUND: u32 = WM_APP + 12;
const NIN_KEYBOARD_SELECT: u32 = NIN_SELECT + 1;

const ID_TRAY: u32 = 1;
const ID_OVERLAY_HIDE_TIMER: usize = 11;
const ID_TRAY_ADD_RETRY_TIMER: usize = 13;
const ID_STARTUP_MUTE_RETRY_TIMER: usize = 14;
const ID_HOTKEY_RECONCILE_TIMER: usize = 15;
const ID_OVERLAY_PREVIEW_LEASE_TIMER: usize = 16;
const ID_CAPTURE_DEVICE_CHANGE_TIMER: usize = 17;
const OVERLAY_PREVIEW_LEASE_MS: u32 = 2_500;
const CAPTURE_DEVICE_CHANGE_DEBOUNCE_MS: u32 = 350;

const ID_MENU_TOGGLE_MUTE: usize = 1001;
const ID_MENU_SETTINGS: usize = 1003;
const ID_MENU_EXIT: usize = 1004;
const ID_MENU_TITLE: usize = 1005;

const SETTINGS_WINDOW_TITLE: &str = "MuteGuard Settings";
const MAIN_INSTANCE_MUTEX: PCWSTR = w!("MuteGuardBackgroundApp");
const DWMWA_MICA_EFFECT: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(1029);
const TRAY_ADD_RETRY_MS: u32 = 2_000;
const STARTUP_MUTE_RETRY_MS: u32 = 2_000;
const HOTKEY_RECONCILE_MS: u32 = 250;
const STARTUP_RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_RUN_VALUE: &str = "MuteGuard";

pub(crate) const HOTKEY_TARGET_ALL_MICROPHONES: &str = "__all_microphones__";
pub(crate) const OVERLAY_DISPLAY_PRIMARY: &str = "Primary";

const VK_SHIFT: u32 = 0x10;
const VK_CONTROL: u32 = 0x11;
const VK_MENU: u32 = 0x12;
const VK_LWIN: u32 = 0x5B;
const VK_RWIN: u32 = 0x5C;
const VK_NUMPAD0: u32 = 0x60;
const VK_F1: u32 = 0x70;
const VK_LBUTTON: u32 = 0x01;
const VK_RBUTTON: u32 = 0x02;
const VK_MBUTTON: u32 = 0x04;
const VK_XBUTTON1: u32 = 0x05;
const VK_XBUTTON2: u32 = 0x06;
const XBUTTON1: u32 = 0x0001;
const XBUTTON2: u32 = 0x0002;
const LLKHF_EXTENDED: u32 = 0x01;
