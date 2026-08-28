pub(crate) fn set_settings_hotkey_recording(recording: bool) {
    let was_recording = SETTINGS_HOTKEY_RECORDING.swap(recording, Ordering::Relaxed);
    if recording != was_recording {
        SETTINGS_MOUSE_HELD.lock().unwrap().clear();
        SETTINGS_MOUSE_CHORD.lock().unwrap().clear();
        SETTINGS_KEYBOARD_HELD.lock().unwrap().clear();
        SETTINGS_KEYBOARD_CHORD.lock().unwrap().clear();
        SETTINGS_CAPTURE_LAST_EVENT.lock().unwrap().take();
        SETTINGS_PRESSED_SHORTCUT.lock().unwrap().take();

        if recording {
            if let Ok(instance) = unsafe { GetModuleHandleW(None) } {
                let _ = install_keyboard_hook(instance.into());
                let _ = install_mouse_hook(instance.into());
            }
        } else {
            remove_settings_capture_hooks();
        }
    }
}

fn remove_settings_capture_hooks() {
    let (keyboard_hook, mouse_hook) = {
        let mut state = STATE.lock().unwrap();
        let hooks = (state.hook, state.mouse_hook);
        state.hook = HHOOK(null_mut());
        state.mouse_hook = HHOOK(null_mut());
        hooks
    };

    if !keyboard_hook.0.is_null() {
        unsafe {
            let _ = UnhookWindowsHookEx(keyboard_hook);
        }
    }
    if !mouse_hook.0.is_null() {
        unsafe {
            let _ = UnhookWindowsHookEx(mouse_hook);
        }
    }
}

pub(crate) fn take_settings_pressed_shortcut() -> Option<Shortcut> {
    poll_settings_shortcut_capture();
    reconcile_settings_capture();
    SETTINGS_PRESSED_SHORTCUT.lock().unwrap().take()
}

fn has_alt_space_hotkey() -> bool {
    STATE
        .lock()
        .unwrap()
        .hotkeys
        .iter()
        .any(|hotkey| shortcut_is_alt_space(&hotkey.shortcut))
}

fn shortcut_is_alt_space(shortcut: &Shortcut) -> bool {
    shortcut.keyboard_keys() == [VK_MENU, 0x20] && shortcut.mouse_buttons.is_empty()
}

pub(crate) fn install_settings_window_guard(hwnd: isize) {
    if hwnd == 0 || SETTINGS_ORIGINAL_WNDPROC.load(Ordering::Relaxed) != 0 {
        return;
    }

    let mica_enabled = load_config()
        .map(|config| effective_settings_mica_enabled(&config))
        .unwrap_or_default();
    SETTINGS_MICA_ENABLED.store(mica_enabled, Ordering::Relaxed);
    apply_settings_backdrop(HWND(hwnd as *mut c_void), mica_enabled);

    let previous = unsafe {
        SetWindowLongPtrW(
            HWND(hwnd as *mut c_void),
            GWL_WNDPROC,
            settings_window_proc as *const () as WindowLongPtrValue,
        )
    };
    if previous != 0 {
        SETTINGS_ORIGINAL_WNDPROC.store(previous as isize, Ordering::Relaxed);
    }
}

fn apply_settings_backdrop(hwnd: HWND, enabled: bool) {
    let enabled = enabled && settings_mica_available();
    unsafe {
        let use_dark_mode = 1_i32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &use_dark_mode as *const _ as *const c_void,
            size_of::<i32>() as u32,
        );

        let backdrop = if enabled {
            DWMSBT_MAINWINDOW
        } else {
            DWMSBT_NONE
        };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const DWM_SYSTEMBACKDROP_TYPE as *const c_void,
            size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );

        let mica_enabled = if enabled { 1_i32 } else { 0_i32 };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_MICA_EFFECT,
            &mica_enabled as *const _ as *const c_void,
            size_of::<i32>() as u32,
        );
    }
}

unsafe extern "system" fn settings_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if matches!(msg, WM_CLOSE | WM_DESTROY) {
        set_settings_hotkey_recording(false);
        request_overlay_preview(false);
    }

    if msg == WM_SYSCOMMAND && (wparam.0 & 0xfff0) == SC_KEYMENU as usize {
        if SETTINGS_HOTKEY_RECORDING.load(Ordering::Relaxed) {
            SETTINGS_PRESSED_SHORTCUT
                .lock()
                .unwrap()
                .replace(Shortcut::from_inputs(vec![VK_MENU, 0x20], Vec::new()));
            return LRESULT(0);
        }
        if has_alt_space_hotkey() {
            return LRESULT(0);
        }
    }

    let refresh_backdrop = matches!(
        msg,
        WM_DWMCOMPOSITIONCHANGED
            | WM_DISPLAYCHANGE
            | WM_DPICHANGED
            | WM_SETTINGCHANGE
            | WM_THEMECHANGED
            | WM_WINDOWPOSCHANGED
    );

    let previous = SETTINGS_ORIGINAL_WNDPROC.load(Ordering::Relaxed);
    let result = if previous == 0 {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    } else {
        let previous: WNDPROC = unsafe { std::mem::transmute(previous) };
        unsafe { CallWindowProcW(previous, hwnd, msg, wparam, lparam) }
    };

    if refresh_backdrop {
        apply_settings_backdrop(hwnd, SETTINGS_MICA_ENABLED.load(Ordering::Relaxed));
    }

    result
}

fn main() {
    install_panic_dialog();
    if let Err(error) = run_entrypoint() {
        show_fatal_error(&format!("MuteGuard could not start:\n\n{error:#}"));
        std::process::exit(1);
    }
}

fn run_entrypoint() -> Result<()> {
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(APP_USER_MODEL_ID_WIDE)
            .context("set MuteGuard application identity")?;
    }
    set_runtime_working_directory()?;
    set_dpi_awareness();

    if let Some(action) = notification_action_from_args(std::env::args().skip(1)) {
        if dispatch_notification_action(action) {
            if action == NotificationAction::ExitAll {
                wait_for_app_shutdown()?;
            }
            return Ok(());
        }
        PENDING_NOTIFICATION_ACTION.lock().unwrap().replace(action);
    }

    if std::env::args().any(|arg| arg == "--settings") {
        let settings_mutex = unsafe { CreateMutexW(None, true, w!("MuteGuardSettingsWindow"))? };
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            focus_settings_window();
            return Ok(());
        }

        if !webview2_runtime_available() {
            show_fatal_error(
                "MuteGuard Settings requires Microsoft Edge WebView2 Runtime.\n\nRun the MuteGuard installer again and accept the WebView2 installation, or install the Evergreen WebView2 Runtime from Microsoft.",
            );
            return Ok(());
        }

        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
        }
        let settings_window_size = LogicalSize::new(1200.0, 740.0);
        let settings_window_min_size = LogicalSize::new(760.0, 590.0);
        let settings_window_position = centered_window_position(settings_window_size);
        let config = DesktopConfig::new()
            .with_window(
                WindowBuilder::new()
                    .with_title(SETTINGS_WINDOW_TITLE)
                    .with_decorations(false)
                    .with_resizable(true)
                    .with_transparent(true)
                    .with_no_redirection_bitmap(true)
                    .with_visible(true)
                    .with_inner_size(settings_window_size)
                    .with_min_inner_size(settings_window_min_size)
                    .with_position(settings_window_position),
            )
            .with_icon(
                dioxus::desktop::icon_from_memory(include_bytes!("../../assets/muteguard.png"))
                    .expect("load app icon"),
            )
            .with_custom_head(gui::settings_startup_head())
            .with_background_color((0, 0, 0, 0));
        MOUSE_HOTKEYS_ENABLED.store(false, Ordering::Relaxed);
        dioxus::LaunchBuilder::desktop()
            .with_cfg(config)
            .launch(gui::settings_app);
        let _settings_mutex = settings_mutex;
        return Ok(());
    }

    let main_mutex = unsafe { CreateMutexW(None, true, MAIN_INSTANCE_MUTEX)? };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let action = PENDING_NOTIFICATION_ACTION
            .lock()
            .unwrap()
            .take()
            .unwrap_or(NotificationAction::OpenSettings);
        anyhow::ensure!(
            dispatch_notification_action_with_retry(action, Duration::from_secs(2)),
            "the running MuteGuard process did not accept the requested action"
        );
        if action == NotificationAction::ExitAll {
            wait_for_app_shutdown()?;
        }
        return Ok(());
    }

    let exit_without_background = {
        let mut pending = PENDING_NOTIFICATION_ACTION.lock().unwrap();
        if *pending == Some(NotificationAction::ExitAll) {
            pending.take();
            true
        } else {
            false
        }
    };
    if exit_without_background {
        close_settings_window();
        wait_for_app_shutdown()?;
        return Ok(());
    }

    let result = run_background_app();
    let _main_mutex = main_mutex;
    result
}

fn set_runtime_working_directory() -> Result<()> {
    let executable = std::env::current_exe().context("locate the MuteGuard executable")?;
    let directory = executable
        .parent()
        .context("locate the MuteGuard application directory")?;
    std::env::set_current_dir(directory).with_context(|| {
        format!(
            "use the MuteGuard application directory {}",
            directory.display()
        )
    })
}

fn install_panic_dialog() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        default_hook(panic_info);
        show_fatal_error(&format!("MuteGuard encountered an unexpected error:\n\n{panic_info}"));
    }));
}

fn show_fatal_error(message: &str) {
    let title = wide("MuteGuard error");
    let message = wide(message);
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn notification_action_from_args(
    args: impl IntoIterator<Item = String>,
) -> Option<NotificationAction> {
    args.into_iter()
        .find_map(|argument| notification_action_from_text(&argument))
}

fn notification_action_from_text(value: &str) -> Option<NotificationAction> {
    let value = value.trim().trim_end_matches('/').to_ascii_lowercase();
    if value == "--toggle-mute"
        || value == "toggle-mute"
        || value.ends_with("://toggle-mute")
    {
        Some(NotificationAction::ToggleMute)
    } else if value == "settings"
        || value == "open-settings"
        || value.ends_with("://settings")
        || value.ends_with("://open-settings")
    {
        Some(NotificationAction::OpenSettings)
    } else if value == "exit-all"
        || value == "--exit-all"
        || value == "quit"
        || value == "--quit"
    {
        Some(NotificationAction::ExitAll)
    } else {
        None
    }
}

#[cfg(test)]
mod notification_action_tests {
    use super::*;

    #[test]
    fn accepts_cli_and_protocol_toggle_actions() {
        assert_eq!(
            notification_action_from_text("--toggle-mute"),
            Some(NotificationAction::ToggleMute)
        );
        assert_eq!(
            notification_action_from_text("muteguard://toggle-mute/"),
            Some(NotificationAction::ToggleMute)
        );
    }

    #[test]
    fn ignores_unknown_notification_actions() {
        assert_eq!(notification_action_from_text("muteguard://unknown"), None);
    }

    #[test]
    fn microphone_notifications_cover_changes_and_disconnects() {
        assert_eq!(
            microphone_change_notification(Some("old"), Some("new"), false, true)
                .map(|(title, _)| title),
            Some("Default communications microphone changed")
        );
        assert_eq!(
            microphone_change_notification(Some("old"), None, false, true)
                .map(|(title, _)| title),
            Some("Microphone disconnected")
        );
        assert_eq!(
            microphone_change_notification(Some("same"), Some("same"), true, true)
                .map(|(title, _)| title),
            Some("Default microphone assignment changed")
        );
    }

    #[test]
    fn microphone_notifications_ignore_duplicates_and_disabled_settings() {
        assert_eq!(
            microphone_change_notification(Some("same"), Some("same"), false, true),
            None
        );
        assert_eq!(
            microphone_change_notification(Some("old"), Some("new"), true, false),
            None
        );
    }
}

fn hidden_window() -> Option<HWND> {
    let class = wide("MuteGuardHidden");
    let hwnd = unsafe { FindWindowW(PCWSTR(class.as_ptr()), PCWSTR(null())) }.ok()?;
    (!hwnd.0.is_null()).then_some(hwnd)
}

fn dispatch_notification_action(action: NotificationAction) -> bool {
    let Some(hwnd) = hidden_window() else {
        return false;
    };
    let message = match action {
        NotificationAction::ToggleMute => WM_TOGGLE_MUTE,
        NotificationAction::OpenSettings => WM_OPEN_SETTINGS,
        NotificationAction::ExitAll => WM_EXIT_ALL,
    };
    let mut message_result = 0_usize;
    unsafe {
        if SendMessageTimeoutW(
            hwnd,
            message,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            2_000,
            Some(&mut message_result),
        )
        .0 == 0
        {
            return false;
        }
    }
    true
}

fn dispatch_notification_action_with_retry(action: NotificationAction, timeout: Duration) -> bool {
    let started = Instant::now();
    loop {
        if dispatch_notification_action(action) {
            return true;
        }
        if started.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for_app_shutdown() -> Result<()> {
    let started = Instant::now();
    while hidden_window().is_some() || settings_window_exists() {
        anyhow::ensure!(
            started.elapsed() < Duration::from_secs(10),
            "MuteGuard did not finish closing within 10 seconds"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn settings_window_exists() -> bool {
    let title = wide(SETTINGS_WINDOW_TITLE);
    unsafe { FindWindowW(PCWSTR(null()), PCWSTR(title.as_ptr())) }
        .is_ok_and(|hwnd| !hwnd.0.is_null())
}

pub(crate) fn request_overlay_preview(enabled: bool) {
    let Some(hwnd) = hidden_window() else {
        return;
    };
    unsafe {
        let _ = PostMessageW(
            hwnd,
            WM_PREVIEW_OVERLAY,
            WPARAM(usize::from(enabled)),
            LPARAM(0),
        );
    }
}

fn request_background_sound_preview(kind: FeedbackKind) -> bool {
    let Some(hwnd) = hidden_window() else {
        return false;
    };
    let kind = match kind {
        FeedbackKind::Mute => 0,
        FeedbackKind::Unmute => 1,
    };
    unsafe { PostMessageW(hwnd, WM_PREVIEW_SOUND, WPARAM(kind), LPARAM(0)).is_ok() }
}

fn apply_pending_notification_action() {
    if let Some(action) = PENDING_NOTIFICATION_ACTION.lock().unwrap().take() {
        handle_notification_action(action);
    }
}

fn handle_notification_action(action: NotificationAction) {
    match action {
        NotificationAction::ToggleMute => toggle_mute(),
        NotificationAction::OpenSettings => open_settings_window(),
        NotificationAction::ExitAll => exit_all_processes(),
    }
}

fn post_audio_window_message(hwnd: HWND, message: u32) -> bool {
    if hwnd.0.is_null() {
        return false;
    }
    unsafe { PostMessageW(hwnd, message, WPARAM(0), LPARAM(0)).is_ok() }
}

fn ensure_audio_notification_registration() -> bool {
    let hwnd = STATE.lock().unwrap().hwnd;
    if hwnd.0.is_null() {
        return false;
    }

    AUDIO_NOTIFICATION_REGISTRATION.with(|registration| {
        let mut registration = registration.borrow_mut();
        if let Some(registration) = registration.as_mut() {
            // Endpoint notifications can represent a disconnect/reconnect that
            // keeps the same Windows device id. Drop the previous volume
            // interface before rebinding so a stale callback is never reused.
            registration.unregister_volume_callback();
            if let Err(error) = registration.rebind_default_capture_volume() {
                eprintln!(
                    "default communications microphone is not ready; retrying notification binding: {error:#}"
                );
                return false;
            }
            return true;
        }

        match AudioNotificationRegistration::new(hwnd) {
            Ok(value) => {
                let bound = value.volume_callback_is_bound();
                *registration = Some(value);
                bound
            }
            Err(error) => {
                report_runtime_error(
                    "MuteGuard could not monitor microphone changes",
                    format!("{error:#}"),
                );
                false
            }
        }
    })
}

fn shutdown_audio_notification_registration() {
    AUDIO_NOTIFICATION_REGISTRATION.with(|registration| {
        if let Some(registration) = registration.borrow_mut().take() {
            registration.shutdown();
        }
    });
}

fn registered_capture_device_id() -> Option<String> {
    AUDIO_NOTIFICATION_REGISTRATION.with(|registration| {
        registration
            .borrow()
            .as_ref()
            .and_then(|registration| registration.device_id.clone())
    })
}

fn schedule_capture_device_change(hwnd: HWND) {
    let timer = unsafe {
        SetTimer(
            hwnd,
            ID_CAPTURE_DEVICE_CHANGE_TIMER,
            CAPTURE_DEVICE_CHANGE_DEBOUNCE_MS,
            None,
        )
    };
    if timer == 0 {
        handle_capture_device_change();
    }
}

fn schedule_capture_device_rebind_retry(hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }
    unsafe {
        let _ = SetTimer(
            hwnd,
            ID_CAPTURE_DEVICE_CHANGE_TIMER,
            CAPTURE_DEVICE_REBIND_RETRY_MS,
            None,
        );
    }
}

fn handle_capture_device_change() {
    let notification_binding_ready = ensure_audio_notification_registration();
    let current_device_id = registered_capture_device_id();
    let current_defaults = default_capture_devices().unwrap_or_default();
    let (previous_device_id, default_selection_changed, notify_changes) = {
        let mut state = STATE.lock().unwrap();
        let previous = std::mem::replace(
            &mut state.last_default_device_id,
            current_device_id.clone(),
        );
        let previous_defaults = std::mem::replace(
            &mut state.last_default_capture_devices,
            current_defaults,
        );
        (
            previous,
            previous_defaults != state.last_default_capture_devices,
            state.device_notifications.notify_changes,
        )
    };
    apply_startup_auto_mute();
    refresh_mute_state_after_device_change();
    if !notification_binding_ready {
        let hwnd = STATE.lock().unwrap().hwnd;
        schedule_capture_device_rebind_retry(hwnd);
    }

    let Some((title, message)) = microphone_change_notification(
        previous_device_id.as_deref(),
        current_device_id.as_deref(),
        default_selection_changed,
        notify_changes,
    ) else {
        return;
    };
    show_tray_info(title, message);
}

fn microphone_change_notification(
    previous_device_id: Option<&str>,
    current_device_id: Option<&str>,
    default_selection_changed: bool,
    enabled: bool,
) -> Option<(&'static str, &'static str)> {
    if !enabled {
        return None;
    }
    if previous_device_id != current_device_id {
        return Some(if current_device_id.is_some() {
            (
                "Default communications microphone changed",
                "Windows changed the default communications microphone. MuteGuard is now monitoring the new device.",
            )
        } else {
            (
                "Microphone disconnected",
                "The default communications microphone is unavailable. MuteGuard will reconnect automatically.",
            )
        });
    }
    default_selection_changed.then_some((
        "Default microphone assignment changed",
        "Windows changed a default microphone assignment. MuteGuard continues monitoring the default communications microphone.",
    ))
}

fn run_background_app() -> Result<()> {
    let startup_registration_error = load_config().ok().and_then(|config| {
        sync_startup_registration(config.startup.launch_on_startup)
            .context("reconcile MuteGuard Windows startup registration")
            .err()
    });

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    let instance = unsafe { GetModuleHandleW(None)? };
    register_class(instance.into())?;
    let hwnd = create_message_window(instance.into())?;
    let mute_state = current_mute_state();
    let muted = mute_state.as_ref().copied().unwrap_or(false);
    let overlay_config = STATE.lock().unwrap().overlay.clone();
    {
        let mut state = STATE.lock().unwrap();
        state.hwnd = hwnd;
        state.muted = muted;
        state.audio_available = mute_state.is_ok();
    }

    native_overlay::init(instance.into(), muted, &overlay_config)?;
    apply_overlay_visibility();
    install_keyboard_hook(instance.into())?;
    install_mouse_hook(instance.into())?;
    add_tray_icon(hwnd)?;
    let automatic_updates_enabled = load_config()
        .map(|config| config.updates.check_automatically)
        .unwrap_or_default();
    schedule_automatic_update_check(automatic_updates_enabled);
    if let Some(error) = startup_registration_error {
        report_runtime_error("MuteGuard could not update Windows startup", format!("{error:#}"));
    }
    if let Err(error) = mute_state {
        report_audio_error("MuteGuard did not find an available microphone", &error);
    }
    let initial_config_error = STATE.lock().unwrap().initial_config_error.take();
    if let Some(error) = initial_config_error {
        report_runtime_error("MuteGuard settings need attention", error);
    }
    if !ensure_audio_notification_registration() {
        schedule_capture_device_rebind_retry(hwnd);
    }
    {
        let mut state = STATE.lock().unwrap();
        state.last_default_device_id = registered_capture_device_id();
        state.last_default_capture_devices = default_capture_devices().unwrap_or_default();
    }
    apply_startup_auto_mute();
    apply_pending_notification_action();
    let message_result = unsafe {
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0);
            if result.0 == -1 {
                break Err(windows::core::Error::from_win32())
                    .context("read the Windows message queue");
            }
            if result.0 == 0 {
                break Ok(());
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    };

    cleanup();
    message_result
}

fn register_class(instance: HINSTANCE) -> Result<()> {
    unsafe {
        RegisterClassW(&WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance,
            lpszClassName: w!("MuteGuardHidden"),
            lpfnWndProc: Some(main_wnd_proc),
            ..Default::default()
        });
    }
    Ok(())
}

fn create_message_window(instance: HINSTANCE) -> Result<HWND> {
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("MuteGuardHidden"),
            w!("MuteGuardHidden"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        )
    }?;
    if hwnd.0.is_null() {
        anyhow::bail!("failed to create hidden MuteGuard window");
    }
    Ok(hwnd)
}

fn install_keyboard_hook(instance: HINSTANCE) -> Result<()> {
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), instance, 0) }
        .context("install low-level keyboard hook")?;
    STATE.lock().unwrap().hook = hook;
    Ok(())
}

fn install_mouse_hook(instance: HINSTANCE) -> Result<()> {
    let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), instance, 0) }
        .context("install low-level mouse hook")?;
    STATE.lock().unwrap().mouse_hook = hook;
    Ok(())
}

fn centered_window_position(size: LogicalSize<f64>) -> PhysicalPosition<i32> {
    let dpi_scale = unsafe { GetDpiForSystem() }.max(96) as f64 / 96.0;
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let window_width = (size.width * dpi_scale).round() as i32;
    let window_height = (size.height * dpi_scale).round() as i32;

    PhysicalPosition::new(
        ((screen_width - window_width) / 2).max(0),
        ((screen_height - window_height) / 2).max(0),
    )
}

fn set_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}
