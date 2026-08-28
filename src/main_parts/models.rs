#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Shortcut {
    #[serde(default, skip_serializing_if = "is_false")]
    ctrl: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    alt: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    shift: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    win: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    vk: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    keyboard_keys: Vec<u32>,
    #[serde(default)]
    mouse_buttons: Vec<u32>,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: 0,
            keyboard_keys: vec![VK_CONTROL, VK_MENU, b'M' as u32],
            mouse_buttons: Vec::new(),
        }
    }
}

impl Shortcut {
    pub(crate) fn empty() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: 0,
            keyboard_keys: Vec::new(),
            mouse_buttons: Vec::new(),
        }
    }

    fn from_inputs(keyboard_keys: Vec<u32>, mouse_buttons: Vec<u32>) -> Self {
        Self {
            keyboard_keys,
            mouse_buttons,
            ..Self::empty()
        }
        .normalized()
    }

    fn keyboard_keys(&self) -> Vec<u32> {
        if !self.keyboard_keys.is_empty() {
            return self.keyboard_keys.clone();
        }

        let mut keys = Vec::new();
        if self.ctrl {
            keys.push(VK_CONTROL);
        }
        if self.alt {
            keys.push(VK_MENU);
        }
        if self.shift {
            keys.push(VK_SHIFT);
        }
        if self.win {
            keys.push(VK_LWIN);
        }
        if self.vk != 0 {
            keys.push(canonical_keyboard_vk(self.vk));
        }
        keys
    }

    fn normalized(mut self) -> Self {
        self.keyboard_keys = self
            .keyboard_keys()
            .into_iter()
            .map(canonical_keyboard_vk)
            .filter(|vk| *vk != 0)
            .collect();
        deduplicate_preserving_order(&mut self.keyboard_keys);
        self.keyboard_keys
            .sort_by_key(|key| keyboard_key_sort_key(*key));
        self.mouse_buttons
            .retain(|button| is_supported_mouse_button(*button));
        self.mouse_buttons
            .sort_by_key(|button| mouse_button_sort_key(*button));
        self.mouse_buttons.dedup();
        self.ctrl = false;
        self.alt = false;
        self.shift = false;
        self.win = false;
        self.vk = 0;
        self
    }

    fn is_pressed(
        &self,
        vk: u32,
        ignore_modifiers: bool,
        keyboard_keys_down: &HashSet<u32>,
        mouse_buttons_down: &HashSet<u32>,
    ) -> bool {
        let keyboard_keys = self.keyboard_keys();
        if keyboard_keys.is_empty() && self.mouse_buttons.is_empty() {
            return false;
        }
        let event_vk = canonical_keyboard_vk(vk);
        (keyboard_keys.contains(&event_vk) || self.mouse_buttons.contains(&vk))
            && self.is_held(
                ignore_modifiers,
                keyboard_keys_down,
                mouse_buttons_down,
            )
    }

    fn is_held(
        &self,
        ignore_modifiers: bool,
        keyboard_keys_down: &HashSet<u32>,
        mouse_buttons_down: &HashSet<u32>,
    ) -> bool {
        let keyboard_keys = self.keyboard_keys();
        if keyboard_keys.is_empty() && self.mouse_buttons.is_empty() {
            return false;
        }
        if !keyboard_keys
            .iter()
            .all(|key| keyboard_keys_down.contains(key))
            || !self
                .mouse_buttons
                .iter()
                .all(|button| mouse_buttons_down.contains(button))
        {
            return false;
        }
        if ignore_modifiers {
            return true;
        }

        [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN]
            .into_iter()
            .all(|modifier| {
                keyboard_keys.contains(&modifier) == keyboard_keys_down.contains(&modifier)
            })
    }

    fn same_inputs(&self, other: &Self) -> bool {
        self.keyboard_keys() == other.keyboard_keys() && self.mouse_buttons == other.mouse_buttons
    }

    fn contains_keyboard_key(&self, vk: u32) -> bool {
        self.keyboard_keys().contains(&canonical_keyboard_vk(vk))
    }

    fn display(&self) -> String {
        let parts = self.parts();
        if parts.is_empty() {
            "None".to_string()
        } else {
            parts.join(" + ")
        }
    }

    pub fn parts(&self) -> Vec<String> {
        let mut parts = self
            .keyboard_keys()
            .into_iter()
            .map(vk_name)
            .collect::<Vec<_>>();
        for button in &self.mouse_buttons {
            parts.push(mouse_button_name(*button).to_string());
        }
        parts
    }
}

fn deduplicate_preserving_order(values: &mut Vec<u32>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(*value));
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HotkeyBinding {
    #[serde(default = "default_hotkey_id")]
    pub id: String,
    #[serde(default)]
    pub shortcut: Shortcut,
    #[serde(default)]
    pub action: HotkeyAction,
    #[serde(default)]
    pub ignore_modifiers: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum HotkeyAction {
    #[default]
    #[serde(rename = "ToggleMute", alias = "toggle", alias = "toggle_mute")]
    ToggleMute,
    #[serde(rename = "Mute", alias = "mute")]
    Mute,
    #[serde(rename = "Unmute", alias = "unmute")]
    Unmute,
}

impl HotkeyAction {
    pub(crate) const fn config_value(self) -> &'static str {
        match self {
            Self::ToggleMute => "ToggleMute",
            Self::Mute => "Mute",
            Self::Unmute => "Unmute",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ToggleMute => "Toggle mute",
            Self::Mute => "Mute",
            Self::Unmute => "Unmute",
        }
    }

    pub(crate) fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "ToggleMute" => Some(Self::ToggleMute),
            "Mute" => Some(Self::Mute),
            "Unmute" => Some(Self::Unmute),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureDeviceOption {
    pub id: String,
    pub name: String,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            id: default_hotkey_id(),
            shortcut: Shortcut::default(),
            action: HotkeyAction::default(),
            ignore_modifiers: false,
            target: None,
        }
    }
}

fn default_hotkey_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = NEXT_HOTKEY_ID.fetch_add(1, Ordering::Relaxed);
    format!("hotkey-{}-{nanos}-{sequence}", std::process::id())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct Config {
    #[serde(default = "default_hotkeys")]
    hotkeys: Vec<HotkeyBinding>,
    #[serde(default)]
    startup: StartupSettings,
    #[serde(default)]
    appearance: AppearanceSettings,
    #[serde(default)]
    overlay: OverlayConfig,
    #[serde(default)]
    tray_icon: TrayIconConfig,
    #[serde(default)]
    device_notifications: DeviceNotificationSettings,
    #[serde(default)]
    updates: UpdateSettings,
    #[serde(default)]
    sound_feedback: SoundFeedbackSettings,
    #[serde(default)]
    advanced: AdvancedSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkeys: default_hotkeys(),
            startup: StartupSettings::default(),
            appearance: AppearanceSettings::default(),
            overlay: OverlayConfig::default(),
            tray_icon: TrayIconConfig::default(),
            device_notifications: DeviceNotificationSettings::default(),
            updates: UpdateSettings::default(),
            sound_feedback: SoundFeedbackSettings::default(),
            advanced: AdvancedSettings::default(),
        }
    }
}

fn default_hotkeys() -> Vec<HotkeyBinding> {
    vec![HotkeyBinding::default()]
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StartupSettings {
    #[serde(default = "default_launch_on_startup")]
    pub launch_on_startup: bool,
    #[serde(default)]
    pub mute_on_startup: bool,
}

impl Default for StartupSettings {
    fn default() -> Self {
        Self {
            launch_on_startup: default_launch_on_startup(),
            mute_on_startup: false,
        }
    }
}

fn default_launch_on_startup() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppearanceSettings {
    #[serde(default = "default_app_accent_style")]
    pub accent_style: String,
    #[serde(default = "default_app_accent_color")]
    pub accent_color: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            accent_style: default_app_accent_style(),
            accent_color: default_app_accent_color(),
        }
    }
}

fn default_app_accent_style() -> String {
    "SystemColor".to_string()
}

fn default_app_accent_color() -> String {
    "#7d42fb".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DeviceNotificationSettings {
    #[serde(default = "default_notify_device_changes")]
    pub notify_changes: bool,
}

impl Default for DeviceNotificationSettings {
    fn default() -> Self {
        Self {
            notify_changes: default_notify_device_changes(),
        }
    }
}

fn default_notify_device_changes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UpdateSettings {
    #[serde(default = "default_check_for_updates")]
    pub check_automatically: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_automatically: default_check_for_updates(),
        }
    }
}

fn default_check_for_updates() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SoundFeedbackSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_sound_feedback_volume")]
    pub volume: u8,
    #[serde(default = "default_sound_source")]
    pub mute_source: String,
    #[serde(default = "default_sound_source")]
    pub unmute_source: String,
}

impl Default for SoundFeedbackSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: default_sound_feedback_volume(),
            mute_source: default_sound_source(),
            unmute_source: default_sound_source(),
        }
    }
}

fn default_sound_feedback_volume() -> u8 {
    45
}

fn default_sound_source() -> String {
    "Default".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AdvancedSettings {
    #[serde(default = "default_enable_mica")]
    pub enable_mica: bool,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            enable_mica: default_enable_mica(),
        }
    }
}

fn default_enable_mica() -> bool {
    settings_mica_available()
}

pub(crate) fn settings_mica_available() -> bool {
    windows_build_number().is_some_and(|build| build >= 22_000)
}

pub(crate) fn effective_settings_mica_enabled(config: &Config) -> bool {
    config.advanced.enable_mica && settings_mica_available()
}

fn windows_build_number() -> Option<u32> {
    let mut data = [0_u16; 32];
    let mut data_size = (data.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(data.as_mut_ptr() as *mut c_void),
            Some(&mut data_size),
        )
    };
    if status != ERROR_SUCCESS || data_size < size_of::<u16>() as u32 {
        return None;
    }
    let len = data
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(data.len());
    String::from_utf16_lossy(&data[..len]).parse().ok()
}

fn webview2_runtime_available() -> bool {
    const WEBVIEW2_CLIENT: &str =
        r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    const WEBVIEW2_CLIENT_WOW64: &str =
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

    [
        (HKEY_LOCAL_MACHINE, WEBVIEW2_CLIENT_WOW64),
        (HKEY_LOCAL_MACHINE, WEBVIEW2_CLIENT),
        (HKEY_CURRENT_USER, WEBVIEW2_CLIENT),
    ]
    .into_iter()
    .filter_map(|(root, path)| read_registry_string(root, path, "pv"))
    .any(|version| !version.is_empty() && version != "0.0.0.0")
}

fn read_registry_string(root: HKEY, path: &str, value_name: &str) -> Option<String> {
    let path = wide(path);
    let value_name = wide(value_name);
    let mut data = [0_u16; 128];
    let mut data_size = (data.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            root,
            PCWSTR(path.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(data.as_mut_ptr() as *mut c_void),
            Some(&mut data_size),
        )
    };
    if status != ERROR_SUCCESS || data_size < size_of::<u16>() as u32 {
        return None;
    }
    let len = data
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(data.len());
    Some(String::from_utf16_lossy(&data[..len]))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct DefaultCaptureDevices {
    communications: Option<String>,
    console: Option<String>,
    multimedia: Option<String>,
}

struct AppState {
    hwnd: HWND,
    hook: HHOOK,
    mouse_hook: HHOOK,
    hotkeys: Vec<HotkeyBinding>,
    mute_on_startup: bool,
    startup_mute_pending: bool,
    overlay: OverlayConfig,
    overlay_preview_active: bool,
    tray_icon: TrayIconConfig,
    notification_tray_icon: Option<HICON>,
    device_notifications: DeviceNotificationSettings,
    sound_feedback: SoundFeedbackSettings,
    last_default_device_id: Option<String>,
    last_default_capture_devices: DefaultCaptureDevices,
    muted: bool,
    audio_available: bool,
    initial_config_error: Option<String>,
    last_error_notification: Option<(String, Instant)>,
    hotkeys_down: HashSet<String>,
    keyboard_keys_down: HashSet<u32>,
    mouse_buttons_down: HashSet<u32>,
    pending_mute_commands: VecDeque<MuteCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MuteCommand {
    action: HotkeyAction,
    target: Option<String>,
}

impl From<HotkeyBinding> for MuteCommand {
    fn from(binding: HotkeyBinding) -> Self {
        Self {
            action: binding.action,
            target: binding.target,
        }
    }
}

struct AudioNotificationRegistration {
    enumerator: IMMDeviceEnumerator,
    endpoint_callback: IMMNotificationClient,
    volume_callback: IAudioEndpointVolumeCallback,
    volume: Option<IAudioEndpointVolume>,
    device_id: Option<String>,
}

impl AudioNotificationRegistration {
    fn new(hwnd: HWND) -> Result<Self> {
        let enumerator = audio_device_enumerator()?;
        let endpoint_callback: IMMNotificationClient =
            windows_core::ComObject::new(AudioDeviceNotificationSink { hwnd }).into_interface();
        let volume_callback: IAudioEndpointVolumeCallback =
            windows_core::ComObject::new(AudioEndpointVolumeSink { hwnd }).into_interface();
        unsafe {
            enumerator
                .RegisterEndpointNotificationCallback(&endpoint_callback)
                .context("register audio endpoint notification callback")?;
        }

        let mut registration = Self {
            enumerator,
            endpoint_callback,
            volume_callback,
            volume: None,
            device_id: None,
        };
        if let Err(error) = registration.rebind_default_capture_volume() {
            eprintln!(
                "default capture endpoint is not ready; waiting for a device notification: {error:?}"
            );
        }
        Ok(registration)
    }

    fn rebind_default_capture_volume(&mut self) -> Result<()> {
        let device = unsafe { capture_device(&self.enumerator)? };
        let device_id = unsafe { endpoint_device_id(&device)? };
        if self.device_id.as_deref() == Some(device_id.as_str()) {
            return Ok(());
        }

        let volume: IAudioEndpointVolume = unsafe {
            device
                .Activate(CLSCTX_ALL, None)
                .context("activate endpoint volume for mute notifications")?
        };
        unsafe {
            volume
                .RegisterControlChangeNotify(&self.volume_callback)
                .context("register endpoint mute notification callback")?;
        }

        if let Some(previous_volume) = self.volume.replace(volume) {
            unsafe {
                if let Err(error) =
                    previous_volume.UnregisterControlChangeNotify(&self.volume_callback)
                {
                    eprintln!("failed to unregister stale endpoint mute callback: {error:?}");
                }
            }
        }
        self.device_id = Some(device_id);
        Ok(())
    }

    fn volume_callback_is_bound(&self) -> bool {
        self.volume.is_some() && self.device_id.is_some()
    }

    fn shutdown(mut self) {
        self.unregister_volume_callback();
        self.unregister_endpoint_callback();
    }

    fn unregister_volume_callback(&mut self) {
        if let Some(volume) = self.volume.take() {
            unsafe {
                if let Err(error) = volume.UnregisterControlChangeNotify(&self.volume_callback) {
                    eprintln!("failed to unregister endpoint mute callback: {error:?}");
                }
            }
        }
        self.device_id = None;
    }

    fn unregister_endpoint_callback(&self) {
        unsafe {
            if let Err(error) = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.endpoint_callback)
            {
                eprintln!("failed to unregister audio endpoint callback: {error:?}");
            }
        }
    }
}

#[windows_core::implement(IAudioEndpointVolumeCallback)]
struct AudioEndpointVolumeSink {
    hwnd: HWND,
}

impl windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback_Impl
    for AudioEndpointVolumeSink_Impl
{
    fn OnNotify(&self, _notification: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        post_audio_window_message(self.hwnd, WM_AUDIO_MUTE_STATE_CHANGED);
        Ok(())
    }
}

#[windows_core::implement(IMMNotificationClient)]
struct AudioDeviceNotificationSink {
    hwnd: HWND,
}

impl AudioDeviceNotificationSink_Impl {
    fn post_rebind(&self) {
        post_audio_window_message(self.hwnd, WM_AUDIO_ENDPOINT_CHANGED);
    }
}

impl windows::Win32::Media::Audio::IMMNotificationClient_Impl for AudioDeviceNotificationSink_Impl {
    fn OnDeviceStateChanged(
        &self,
        _device_id: &PCWSTR,
        _new_state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        self.post_rebind();
        Ok(())
    }

    fn OnDeviceAdded(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        self.post_rebind();
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device_id: &PCWSTR) -> windows::core::Result<()> {
        self.post_rebind();
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: EDataFlow,
        _role: ERole,
        _device_id: &PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == eCapture {
            post_audio_window_message(self.hwnd, WM_DEFAULT_CAPTURE_DEVICE_CHANGED);
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device_id: &PCWSTR,
        _key: &windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NotificationAction {
    ToggleMute,
    Mute,
    Unmute,
    OpenSettings,
    ExitAll,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OverlayConfig {
    #[serde(default = "default_overlay_enabled")]
    pub enabled: bool,
    #[serde(default = "default_overlay_visibility")]
    pub visibility: String,
    #[serde(default = "default_overlay_display")]
    pub display: String,
    #[serde(default = "default_overlay_displays")]
    pub displays: Vec<String>,
    #[serde(default = "default_overlay_position_x")]
    pub position_x: f64,
    #[serde(default = "default_overlay_position_y")]
    pub position_y: f64,
    #[serde(default = "default_overlay_duration_secs")]
    pub duration_secs: f64,
    #[serde(default = "default_overlay_scale")]
    pub scale: u32,
    #[serde(default)]
    pub show_text: bool,
    #[serde(default = "default_overlay_muted_label")]
    pub muted_label: String,
    #[serde(default = "default_overlay_unmuted_label")]
    pub unmuted_label: String,
    #[serde(default = "default_overlay_text_font")]
    pub text_font: String,
    #[serde(default = "default_overlay_text_font_weight")]
    pub text_font_weight: u16,
    #[serde(default = "default_overlay_variant")]
    pub variant: String,
    #[serde(default = "crate::overlay_icons::default_overlay_icon_pair")]
    pub icon_pair: String,
    #[serde(default = "default_overlay_icon_style")]
    pub icon_style: String,
    #[serde(default = "default_overlay_icon_color")]
    pub icon_color: String,
    #[serde(default = "default_overlay_background_style")]
    pub background_style: String,
    #[serde(default = "default_overlay_background_opacity")]
    pub background_opacity: u8,
    #[serde(default = "default_overlay_content_opacity")]
    pub content_opacity: u8,
    #[serde(default = "default_overlay_border_radius")]
    pub border_radius: u8,
    #[serde(default = "default_overlay_show_border")]
    pub show_border: bool,
    #[serde(default = "default_overlay_border_color")]
    pub border_color: String,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: default_overlay_enabled(),
            visibility: default_overlay_visibility(),
            display: default_overlay_display(),
            displays: default_overlay_displays(),
            position_x: default_overlay_position_x(),
            position_y: default_overlay_position_y(),
            duration_secs: default_overlay_duration_secs(),
            scale: default_overlay_scale(),
            show_text: false,
            muted_label: default_overlay_muted_label(),
            unmuted_label: default_overlay_unmuted_label(),
            text_font: default_overlay_text_font(),
            text_font_weight: default_overlay_text_font_weight(),
            variant: default_overlay_variant(),
            icon_pair: crate::overlay_icons::default_overlay_icon_pair(),
            icon_style: default_overlay_icon_style(),
            icon_color: default_overlay_icon_color(),
            background_style: default_overlay_background_style(),
            background_opacity: default_overlay_background_opacity(),
            content_opacity: default_overlay_content_opacity(),
            border_radius: default_overlay_border_radius(),
            show_border: default_overlay_show_border(),
            border_color: default_overlay_border_color(),
        }
    }
}

fn default_overlay_enabled() -> bool { true }
fn default_overlay_visibility() -> String { "WhenMuted".to_string() }
fn default_overlay_display() -> String { OVERLAY_DISPLAY_PRIMARY.to_string() }
fn default_overlay_displays() -> Vec<String> { vec![default_overlay_display()] }
fn default_overlay_position_x() -> f64 { 50.0 }
fn default_overlay_position_y() -> f64 { 0.0 }
fn default_overlay_duration_secs() -> f64 { 2.0 }
fn default_overlay_scale() -> u32 { 100 }
fn default_overlay_muted_label() -> String { "Microphone muted".to_string() }
fn default_overlay_unmuted_label() -> String { "Microphone on".to_string() }
fn default_overlay_text_font() -> String { "Segoe UI".to_string() }
fn default_overlay_text_font_weight() -> u16 { 700 }
fn default_overlay_variant() -> String { "MicIcon".to_string() }
fn default_overlay_icon_style() -> String { "Custom".to_string() }
fn default_overlay_icon_color() -> String { "#7c83ff".to_string() }
fn default_overlay_background_style() -> String { "Dark".to_string() }
fn default_overlay_background_opacity() -> u8 { 90 }
fn default_overlay_content_opacity() -> u8 { 100 }
fn default_overlay_border_radius() -> u8 { 6 }
fn default_overlay_show_border() -> bool { true }
fn default_overlay_border_color() -> String { "#323441".to_string() }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayDisplay {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub primary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFont {
    pub family: String,
}

fn normalize_overlay_font_family(family: &str) -> String {
    let family = family.trim();
    if [
        "Segoe UI Light",
        "Segoe UI Semilight",
        "Segoe UI Semibold",
        "Segoe UI Black",
    ]
    .iter()
    .any(|styled_family| family.eq_ignore_ascii_case(styled_family))
    {
        "Segoe UI".to_string()
    } else if family.is_empty() {
        default_overlay_text_font()
    } else {
        family.to_string()
    }
}

pub fn system_fonts() -> Vec<SystemFont> {
    let mut families = Vec::<String>::new();
    unsafe {
        let hdc = CreateCompatibleDC(None);
        if !hdc.0.is_null() {
            let logfont = LOGFONTW {
                lfCharSet: DEFAULT_CHARSET,
                ..Default::default()
            };
            let _ = EnumFontFamiliesExW(
                hdc,
                &logfont,
                Some(collect_system_font),
                LPARAM(&mut families as *mut _ as isize),
                0,
            );
            let _ = DeleteDC(hdc);
        }
    }
    families = families
        .into_iter()
        .map(|family| normalize_overlay_font_family(&family))
        .collect();
    families.sort_by_key(|family| family.to_ascii_lowercase());
    families.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    if families.is_empty() {
        families = vec!["Segoe UI".to_string(), "Arial".to_string(), "Tahoma".to_string()];
    }
    families
        .into_iter()
        .map(|family| SystemFont { family })
        .collect()
}

unsafe extern "system" fn collect_system_font(
    logfont: *const LOGFONTW,
    _text_metric: *const TEXTMETRICW,
    _font_type: u32,
    data: LPARAM,
) -> i32 {
    if logfont.is_null() || data.0 == 0 {
        return 1;
    }
    let families = unsafe { &mut *(data.0 as *mut Vec<String>) };
    let face = unsafe { wide_buf_to_string(&(*logfont).lfFaceName) };
    if !face.is_empty() && !face.starts_with('@') {
        families.push(face);
    }
    1
}

fn wide_buf_to_string(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len]).trim().to_string()
}

pub fn overlay_displays() -> Vec<OverlayDisplay> {
    let mut monitors = Vec::<MonitorSnapshot>::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(collect_monitor_snapshot),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    if monitors.is_empty() {
        return vec![OverlayDisplay {
            id: OVERLAY_DISPLAY_PRIMARY.to_string(),
            label: "Primary display".to_string(),
            detail: "Windows primary monitor".to_string(),
            primary: true,
        }];
    }
    monitors
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let number = index + 1;
            let width = monitor.rect.right - monitor.rect.left;
            let height = monitor.rect.bottom - monitor.rect.top;
            OverlayDisplay {
                id: if monitor.primary {
                    OVERLAY_DISPLAY_PRIMARY.to_string()
                } else {
                    monitor.device_name
                },
                label: if monitor.primary {
                    format!("Display {number} (primary)")
                } else {
                    format!("Display {number}")
                },
                detail: format!("{width} × {height}"),
                primary: monitor.primary,
            }
        })
        .collect()
}

#[derive(Clone)]
struct MonitorSnapshot {
    rect: RECT,
    primary: bool,
    device_name: String,
}

unsafe extern "system" fn collect_monitor_snapshot(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(data.0 as *mut Vec<MonitorSnapshot>) };
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
        monitors.push(MonitorSnapshot {
            rect: info.monitorInfo.rcMonitor,
            primary: (info.monitorInfo.dwFlags & 1) != 0,
            device_name: wide_buf_to_string(&info.szDevice),
        });
    }
    true.into()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TrayIconConfig {
    #[serde(default = "default_tray_icon_variant")]
    pub variant: String,
    #[serde(default = "crate::overlay_icons::default_overlay_icon_pair")]
    pub icon_pair: String,
    #[serde(default = "default_tray_icon_status_style")]
    pub status_style: String,
    #[serde(default = "default_tray_icon_status_color")]
    pub status_color: String,
}

impl Default for TrayIconConfig {
    fn default() -> Self {
        Self {
            variant: default_tray_icon_variant(),
            icon_pair: crate::overlay_icons::default_overlay_icon_pair(),
            status_style: default_tray_icon_status_style(),
            status_color: default_tray_icon_status_color(),
        }
    }
}

fn default_tray_icon_variant() -> String { "StatusMic".to_string() }
fn default_tray_icon_status_style() -> String { "Custom".to_string() }
fn default_tray_icon_status_color() -> String { "#7c83ff".to_string() }

#[derive(Clone, Copy, Debug)]
pub struct WindowsAccent {
    accent: (u8, u8, u8),
}

impl Default for WindowsAccent {
    fn default() -> Self {
        Self { accent: (250, 121, 48) }
    }
}

impl WindowsAccent {
    pub fn load() -> Self {
        let fallback = Self::default();
        Self {
            accent: read_windows_accent_dword()
                .map(windows_accent_to_rgb)
                .filter(|accent| *accent != (0, 0, 0))
                .unwrap_or(fallback.accent),
        }
    }

    pub fn css_vars(self) -> String {
        let (red, green, blue) = self.accent;
        format!(":root {{ --windows-accent: rgb({red}, {green}, {blue}); }}")
    }

    pub fn css_color(self) -> String {
        let (red, green, blue) = self.accent;
        format!("rgb({red}, {green}, {blue})")
    }
}

pub(crate) fn effective_app_accent_css(config: &Config) -> String {
    if config.appearance.accent_style == "Custom"
        && let Some((red, green, blue)) = parse_hex_color(&config.appearance.accent_color)
    {
        return format!("rgb({red}, {green}, {blue})");
    }

    WindowsAccent::load().css_color()
}

fn read_windows_accent_dword() -> Option<u32> {
    read_registry_dword(
        w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent"),
        w!("AccentColorMenu"),
    )
    .or_else(|| read_registry_dword(w!(r"Software\Microsoft\Windows\DWM"), w!("AccentColor")))
}

fn read_registry_dword(subkey: PCWSTR, value_name: PCWSTR) -> Option<u32> {
    let mut data = 0_u32;
    let mut data_size = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            value_name,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut _ as *mut c_void),
            Some(&mut data_size),
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

fn windows_accent_to_rgb(value: u32) -> (u8, u8, u8) {
    (
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
    )
}

fn windows_uses_light_system_theme() -> bool {
    read_registry_dword(
        w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
        w!("SystemUsesLightTheme"),
    )
    .unwrap_or(0)
        != 0
}

fn state_accent(muted: bool) -> (u8, u8, u8) {
    if muted { (220, 53, 69) } else { (40, 167, 69) }
}

impl Default for AppState {
    fn default() -> Self {
        let (config, initial_config_error) = match load_config() {
            Ok(config) => (config, None),
            Err(error) => (
                Config::default(),
                Some(format!("MuteGuard could not load its settings: {error:#}")),
            ),
        };
        Self {
            hwnd: HWND(null_mut()),
            hook: HHOOK(null_mut()),
            mouse_hook: HHOOK(null_mut()),
            hotkeys: config.hotkeys,
            mute_on_startup: config.startup.mute_on_startup,
            startup_mute_pending: config.startup.mute_on_startup,
            overlay: config.overlay,
            overlay_preview_active: false,
            tray_icon: config.tray_icon,
            notification_tray_icon: None,
            device_notifications: config.device_notifications,
            sound_feedback: config.sound_feedback,
            last_default_device_id: None,
            last_default_capture_devices: DefaultCaptureDevices::default(),
            muted: false,
            audio_available: false,
            initial_config_error,
            last_error_notification: None,
            hotkeys_down: HashSet::new(),
            keyboard_keys_down: HashSet::new(),
            mouse_buttons_down: HashSet::new(),
            pending_mute_commands: VecDeque::new(),
        }
    }
}

unsafe impl Send for AppState {}
