fn add_tray_icon(hwnd: HWND) -> Result<()> {
    if TRAY_ICON_ADDED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let (config, muted, audio_available) = {
        let state = STATE.lock().unwrap();
        (state.tray_icon.clone(), state.muted, state.audio_available)
    };
    let icon = load_effective_tray_icon(&config, muted, audio_available)
        .context("load MuteGuard tray icon")?;
    let mut nid = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID_TRAY,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: icon,
        ..Default::default()
    };
    write_packed_wide_buf(std::ptr::addr_of_mut!(nid.szTip), "MuteGuard");
    let added = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
    unsafe {
        let _ = DestroyIcon(icon);
    }
    unsafe {
        if added {
            TRAY_ICON_ADDED.store(true, Ordering::Relaxed);
            let _ = KillTimer(hwnd, ID_TRAY_ADD_RETRY_TIMER);
            nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
        } else {
            let _ = SetTimer(hwnd, ID_TRAY_ADD_RETRY_TIMER, TRAY_ADD_RETRY_MS, None);
            return Ok(());
        }
    }
    refresh_tray_icon();
    Ok(())
}

fn refresh_tray_icon() {
    if !TRAY_ICON_ADDED.load(Ordering::Relaxed) {
        return;
    }

    let (hwnd, muted, audio_available, config) = {
        let state = STATE.lock().unwrap();
        if state.hwnd.0.is_null() {
            return;
        }
        (
            state.hwnd,
            state.muted,
            state.audio_available,
            state.tray_icon.clone(),
        )
    };
    let icon = load_effective_tray_icon(&config, muted, audio_available);
    if let Some(icon) = icon {
        let nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: ID_TRAY,
            uFlags: NIF_ICON,
            hIcon: icon,
            ..Default::default()
        };
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
            let _ = DestroyIcon(icon);
        }
    }
    refresh_tray_tip();
}

fn load_tray_icon(config: &TrayIconConfig, muted: bool) -> Option<HICON> {
    match config.variant.as_str() {
        "StatusMic" => create_status_mic_icon(config, muted),
        "ColorDot" => create_color_dot_icon(muted),
        _ => load_app_icon(),
    }
}

fn load_effective_tray_icon(
    config: &TrayIconConfig,
    muted: bool,
    audio_available: bool,
) -> Option<HICON> {
    if audio_available {
        load_tray_icon(config, muted).or_else(load_app_icon)
    } else {
        load_app_icon()
    }
}

fn load_app_icon() -> Option<HICON> {
    const ICON_RESOURCE_VERSION: u32 = 0x0003_0000;
    let icon_bytes = include_bytes!("../../assets/muteguard.ico");
    let image = best_ico_image(icon_bytes, 16)?;
    unsafe {
        CreateIconFromResourceEx(image, true, ICON_RESOURCE_VERSION, 0, 0, LR_DEFAULTSIZE).ok()
    }
}

fn create_status_mic_icon(config: &TrayIconConfig, muted: bool) -> Option<HICON> {
    let color = match config.status_style.as_str() {
        "Monochrome" => {
            if windows_uses_light_system_theme() {
                (0, 0, 0)
            } else {
                (245, 245, 245)
            }
        }
        "SystemColor" => WindowsAccent::load().accent,
        "Custom" => parse_hex_color(&config.status_color).unwrap_or_else(|| state_accent(muted)),
        _ => state_accent(muted),
    };
    let mask = fit_alpha_mask(
        &render_svg_alpha(
            crate::overlay_icons::overlay_icon_svg(&config.icon_pair, muted),
            64,
        )?,
        64,
        64,
        32,
        30,
    )?;
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for (index, alpha) in mask.into_iter().enumerate() {
        let offset = index * 4;
        pixels[offset..offset + 4].copy_from_slice(&premultiplied_bgra(color, alpha));
    }
    create_argb_icon(32, 32, &pixels)
}

fn create_color_dot_icon(muted: bool) -> Option<HICON> {
    let color = state_accent(muted);
    let size = 32usize;
    let center = (size as f64 - 1.0) / 2.0;
    let radius = 13.25;
    let feather = 1.25;
    let mut pixels = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let distance = ((x as f64 - center).powi(2) + (y as f64 - center).powi(2)).sqrt();
            let alpha = ((radius + feather - distance) / feather).clamp(0.0, 1.0);
            let offset = (y * size + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&premultiplied_bgra(
                color,
                (alpha * 255.0).round() as u8,
            ));
        }
    }
    create_argb_icon(size as i32, size as i32, &pixels)
}

fn premultiplied_bgra(color: (u8, u8, u8), alpha: u8) -> [u8; 4] {
    let premultiply = |channel: u8| {
        ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
    };
    [
        premultiply(color.2),
        premultiply(color.1),
        premultiply(color.0),
        alpha,
    ]
}

fn render_svg_alpha(svg: &str, size: u32) -> Option<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).ok()?;
    let svg_size = tree.size().to_int_size();
    let scale = (size as f32 / svg_size.width() as f32)
        .min(size as f32 / svg_size.height() as f32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let pixels = pixmap.take_demultiplied();
    Some(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[3])
            .collect(),
    )
}

fn fit_alpha_mask(
    mask: &[u8],
    source_width: usize,
    source_height: usize,
    target_size: usize,
    content_size: usize,
) -> Option<Vec<u8>> {
    if source_width == 0
        || source_height == 0
        || target_size == 0
        || content_size == 0
        || mask.len() < source_width * source_height
    {
        return None;
    }

    let mut min_x = source_width;
    let mut min_y = source_height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for y in 0..source_height {
        for x in 0..source_width {
            if mask[y * source_width + x] == 0 {
                continue;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x > max_x || min_y > max_y {
        return Some(vec![0; target_size * target_size]);
    }

    let bounds_width = max_x - min_x + 1;
    let bounds_height = max_y - min_y + 1;
    let fitted_width = if bounds_width >= bounds_height {
        content_size
    } else {
        ((bounds_width as f64 / bounds_height as f64) * content_size as f64)
            .round()
            .max(1.0) as usize
    };
    let fitted_height = if bounds_height >= bounds_width {
        content_size
    } else {
        ((bounds_height as f64 / bounds_width as f64) * content_size as f64)
            .round()
            .max(1.0) as usize
    };
    let offset_x = (target_size.saturating_sub(fitted_width)) / 2;
    let offset_y = (target_size.saturating_sub(fitted_height)) / 2;
    let mut fitted = vec![0; target_size * target_size];

    for y in 0..fitted_height {
        for x in 0..fitted_width {
            let source_x = min_x
                + ((x as f64 + 0.5) * bounds_width as f64 / fitted_width as f64)
                    .floor()
                    .min((bounds_width - 1) as f64) as usize;
            let source_y = min_y
                + ((y as f64 + 0.5) * bounds_height as f64 / fitted_height as f64)
                    .floor()
                    .min((bounds_height - 1) as f64) as usize;
            fitted[(offset_y + y) * target_size + offset_x + x] =
                mask[source_y * source_width + source_x];
        }
    }
    Some(fitted)
}

fn create_argb_icon(width: i32, height: i32, pixels: &[u8]) -> Option<HICON> {
    let and_mask = vec![0u8; ((width * height) / 8).max(1) as usize];
    unsafe {
        CreateIcon(
            None,
            width,
            height,
            1,
            32,
            and_mask.as_ptr(),
            pixels.as_ptr(),
        )
        .ok()
    }
}

fn best_ico_image(bytes: &[u8], target: u32) -> Option<&[u8]> {
    if bytes.len() < 6 || u16::from_le_bytes([bytes[0], bytes[1]]) != 0 {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut best = None;
    for index in 0..count {
        let offset = 6 + index * 16;
        if offset + 16 > bytes.len() {
            break;
        }
        let width = if bytes[offset] == 0 {
            256
        } else {
            bytes[offset] as u32
        };
        let height = if bytes[offset + 1] == 0 {
            256
        } else {
            bytes[offset + 1] as u32
        };
        let size = u32::from_le_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
        ]) as usize;
        let image_offset = u32::from_le_bytes([
            bytes[offset + 12],
            bytes[offset + 13],
            bytes[offset + 14],
            bytes[offset + 15],
        ]) as usize;
        if image_offset.checked_add(size).is_none_or(|end| end > bytes.len()) {
            continue;
        }
        let score = width.abs_diff(target) + height.abs_diff(target);
        if best.is_none_or(|(best_score, best_size, _)| {
                score < best_score || (score == best_score && size > best_size)
            })
        {
            best = Some((score, size, image_offset));
        }
    }
    let (_, size, image_offset) = best?;
    Some(&bytes[image_offset..image_offset + size])
}

fn refresh_tray_tip() {
    let state = STATE.lock().unwrap();
    if state.hwnd.0.is_null() {
        return;
    }
    let mut nid = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: state.hwnd,
        uID: ID_TRAY,
        uFlags: NIF_TIP,
        ..Default::default()
    };
    let state_text = if !state.audio_available {
        "microphone unavailable"
    } else if state.muted {
        "microphone muted"
    } else {
        "microphone on"
    };
    write_packed_wide_buf(
        std::ptr::addr_of_mut!(nid.szTip),
        &format!("MuteGuard - {state_text}"),
    );
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

fn report_runtime_error(title: &str, detail: impl Into<String>) {
    let detail = detail.into();
    eprintln!("{title}: {detail}");
    let should_notify = {
        let mut state = STATE.lock().unwrap();
        let now = Instant::now();
        let recently_reported = state
            .last_error_notification
            .as_ref()
            .is_some_and(|(previous, timestamp)| {
                previous == &detail && now.duration_since(*timestamp) < Duration::from_secs(30)
            });
        if !recently_reported {
            state.last_error_notification = Some((detail.clone(), now));
        }
        !recently_reported
    };

    if should_notify {
        show_tray_error(title, &detail);
    }
}

fn report_audio_error(context: &str, error: &anyhow::Error) {
    mark_audio_unavailable();
    report_runtime_error(context, format!("{error:#}"));
}

fn mark_audio_unavailable() {
    {
        let mut state = STATE.lock().unwrap();
        state.audio_available = false;
    }
    refresh_tray_icon();
    apply_overlay_visibility();
}

fn show_tray_error(title: &str, detail: &str) {
    show_tray_notification(title, detail, NIIF_ERROR);
}

fn show_tray_info(title: &str, detail: &str) {
    show_tray_notification(title, detail, NIIF_INFO);
}

fn show_tray_notification(
    title: &str,
    detail: &str,
    kind: windows::Win32::UI::Shell::NOTIFY_ICON_INFOTIP_FLAGS,
) {
    if let Err(error) = show_app_notification(title, detail) {
        eprintln!("modern MuteGuard notification failed; using tray fallback: {error:#}");
        show_legacy_tray_notification(title, detail, kind);
    }
}

fn show_app_notification(title: &str, detail: &str) -> Result<()> {
    show_app_notification_with_launch(title, detail, "muteguard://settings")
}

fn show_update_notification(title: &str, detail: &str, launch_url: &str) {
    if let Err(error) = show_app_notification_with_launch(title, detail, launch_url) {
        eprintln!("modern MuteGuard update notification failed; using tray fallback: {error:#}");
        show_legacy_tray_notification(title, detail, NIIF_INFO);
    }
}

fn show_app_notification_with_launch(title: &str, detail: &str, launch_url: &str) -> Result<()> {
    let content = XmlDocument::new().context("create notification XML document")?;
    content
        .LoadXml(&HSTRING::from(notification_xml_with_launch(
            title, detail, launch_url,
        )))
        .context("load notification XML")?;
    let notification = ToastNotification::CreateToastNotification(&content)
        .context("create Windows notification")?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
        APP_USER_MODEL_ID,
    ))
    .context("open MuteGuard notification channel")?;
    notifier
        .Show(&notification)
        .context("show Windows notification")
}

fn notification_xml_with_launch(title: &str, detail: &str, launch_url: &str) -> String {
    format!(
        concat!(
            r#"<toast launch="{}" activationType="protocol">"#,
            r#"<visual><binding template="ToastGeneric"><text>{}</text>"#,
            r#"<text>{}</text></binding></visual></toast>"#
        ),
        escape_notification_text(launch_url),
        escape_notification_text(title),
        escape_notification_text(detail),
    )
}

fn escape_notification_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn show_legacy_tray_notification(
    title: &str,
    detail: &str,
    kind: windows::Win32::UI::Shell::NOTIFY_ICON_INFOTIP_FLAGS,
) {
    if !TRAY_ICON_ADDED.load(Ordering::Relaxed) {
        return;
    }
    let hwnd = {
        let state = STATE.lock().unwrap();
        state.hwnd
    };
    if hwnd.0.is_null() {
        return;
    }

    let notification_icon = load_app_icon();
    let previous_notification_icon = {
        let mut state = STATE.lock().unwrap();
        if let Some(icon) = notification_icon {
            state.notification_tray_icon.replace(icon)
        } else {
            state.notification_tray_icon.take()
        }
    };
    if let Some(icon) = previous_notification_icon {
        unsafe {
            let _ = DestroyIcon(icon);
        }
    }

    let mut nid = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID_TRAY,
        uFlags: if notification_icon.is_some() {
            NIF_INFO | NIF_ICON
        } else {
            NIF_INFO
        },
        hIcon: notification_icon.unwrap_or_default(),
        dwInfoFlags: kind,
        ..Default::default()
    };
    write_packed_wide_buf(std::ptr::addr_of_mut!(nid.szInfoTitle), title);
    write_packed_wide_buf(std::ptr::addr_of_mut!(nid.szInfo), detail);
    let delivered = if unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() } {
        true
    } else {
        TRAY_ICON_ADDED.store(false, Ordering::Relaxed);
        if add_tray_icon(hwnd).is_ok() && TRAY_ICON_ADDED.load(Ordering::Relaxed) {
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() }
        } else {
            false
        }
    };
    if !delivered {
        eprintln!("failed to deliver MuteGuard tray notification: {title}");
    }
    refresh_tray_icon();
}

fn remove_tray_icon() {
    let (hwnd, notification_icon) = {
        let mut state = STATE.lock().unwrap();
        (state.hwnd, state.notification_tray_icon.take())
    };
    if let Some(icon) = notification_icon {
        unsafe {
            let _ = DestroyIcon(icon);
        }
    }
    if hwnd.0.is_null() {
        return;
    }
    let nid = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID_TRAY,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
    TRAY_ICON_ADDED.store(false, Ordering::Relaxed);
}

fn handle_sound_preview(value: usize) {
    let kind = if value == 0 {
        FeedbackKind::Mute
    } else {
        FeedbackKind::Unmute
    };
    let settings = STATE.lock().unwrap().sound_feedback.clone();
    queue_sound_preview(kind, &settings);
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == *TASKBAR_CREATED_MESSAGE {
        TRAY_ICON_ADDED.store(false, Ordering::Relaxed);
        let _ = add_tray_icon(hwnd);
        return LRESULT(0);
    }

    match msg {
        WM_TRAY => {
            match lparam.0 as u32 & 0xffff {
                WM_LBUTTONUP | NIN_SELECT | NIN_KEYBOARD_SELECT => open_settings_window(),
                WM_RBUTTONUP | WM_CONTEXTMENU => show_tray_menu(hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            handle_tray_menu_command(wparam.0 & 0xffff);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == ID_OVERLAY_HIDE_TIMER {
                let _ = unsafe { KillTimer(hwnd, ID_OVERLAY_HIDE_TIMER) };
                apply_overlay_visibility();
            } else if wparam.0 == ID_TRAY_ADD_RETRY_TIMER {
                let _ = add_tray_icon(hwnd);
            } else if wparam.0 == ID_STARTUP_MUTE_RETRY_TIMER {
                apply_startup_auto_mute();
            } else if wparam.0 == ID_HOTKEY_RECONCILE_TIMER {
                reconcile_hotkeys_down();
            } else if wparam.0 == ID_OVERLAY_PREVIEW_LEASE_TIMER {
                set_overlay_preview(false);
            } else if wparam.0 == ID_CAPTURE_DEVICE_CHANGE_TIMER {
                let _ = unsafe { KillTimer(hwnd, ID_CAPTURE_DEVICE_CHANGE_TIMER) };
                handle_capture_device_change();
            }
            LRESULT(0)
        }
        WM_TOGGLE_MUTE | WM_MUTE | WM_UNMUTE => {
            handle_microphone_action_message(msg);
            LRESULT(0)
        }
        WM_OPEN_SETTINGS => {
            open_settings_window();
            LRESULT(0)
        }
        WM_EXIT_ALL => {
            exit_all_processes();
            LRESULT(0)
        }
        WM_AUDIO_MUTE_STATE_CHANGED => {
            refresh_mute_state();
            LRESULT(0)
        }
        WM_AUDIO_ENDPOINT_CHANGED => {
            schedule_capture_device_change(hwnd);
            LRESULT(0)
        }
        WM_CONFIG_CHANGED => {
            reload_config_now();
            LRESULT(0)
        }
        WM_PROCESS_HOTKEY_ACTIONS => {
            process_queued_mute_commands();
            LRESULT(0)
        }
        WM_PREVIEW_OVERLAY => {
            set_overlay_preview(wparam.0 != 0);
            LRESULT(0)
        }
        WM_DEFAULT_CAPTURE_DEVICE_CHANGED => {
            schedule_capture_device_change(hwnd);
            LRESULT(0)
        }
        WM_PREVIEW_SOUND => {
            handle_sound_preview(wparam.0);
            LRESULT(0)
        }
        WM_UPDATE_AVAILABLE => {
            notify_available_update();
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            refresh_overlay_displays();
            LRESULT(0)
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED => {
            refresh_tray_icon();
            native_overlay::refresh_system_theme();
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn handle_microphone_action_message(message: u32) {
    match message {
        WM_TOGGLE_MUTE => toggle_mute(),
        WM_MUTE => mute(),
        WM_UNMUTE => unmute(),
        _ => {}
    }
}

fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu().unwrap_or_default();
        if menu.0.is_null() {
            return;
        }
        let (audio_available, muted) = {
            let state = STATE.lock().unwrap();
            (state.audio_available, state.muted)
        };
        let title = wide(&format!("MuteGuard {}", env!("CARGO_PKG_VERSION")));
        let toggle_mute = wide(if !audio_available {
            "Microphone unavailable"
        } else if muted {
            "Unmute microphone"
        } else {
            "Mute microphone"
        });
        let settings = wide("Settings");
        let exit = wide("Exit");

        let _ = AppendMenuW(
            menu,
            MENU_ITEM_FLAGS(0x0000_0001),
            ID_MENU_TITLE,
            PCWSTR(title.as_ptr()),
        );
        let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0x0000_0800), 0, PCWSTR(null()));
        let _ = AppendMenuW(
            menu,
            MENU_ITEM_FLAGS(if audio_available { 0 } else { 0x0000_0001 }),
            ID_MENU_TOGGLE_MUTE,
            PCWSTR(toggle_mute.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MENU_ITEM_FLAGS(0),
            ID_MENU_SETTINGS,
            PCWSTR(settings.as_ptr()),
        );
        let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0x0000_0800), 0, PCWSTR(null()));
        let _ = AppendMenuW(
            menu,
            MENU_ITEM_FLAGS(0),
            ID_MENU_EXIT,
            PCWSTR(exit.as_ptr()),
        );

        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            None,
        );
        if command.0 != 0 {
            handle_tray_menu_command(command.0 as usize);
        }
        let _ = DestroyMenu(menu);
    }
}

fn handle_tray_menu_command(command_id: usize) {
    match command_id {
        ID_MENU_TOGGLE_MUTE => toggle_mute(),
        ID_MENU_SETTINGS => open_settings_window(),
        ID_MENU_EXIT => exit_all_processes(),
        _ => {}
    }
}

fn open_settings_window() {
    if !focus_settings_window() {
        launch_settings_window();
    }
}

fn launch_settings_window() {
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            report_runtime_error(
                "MuteGuard could not open Settings",
                format!("Could not locate the running executable: {error}"),
            );
            return;
        }
    };
    if let Err(error) = Command::new(executable).arg("--settings").spawn() {
        report_runtime_error(
            "MuteGuard could not open Settings",
            format!("Could not start the Settings process: {error}"),
        );
    }
}

fn focus_settings_window() -> bool {
    let title = wide(SETTINGS_WINDOW_TITLE);
    let Ok(hwnd) = (unsafe { FindWindowW(PCWSTR(null()), PCWSTR(title.as_ptr())) }) else {
        return false;
    };
    if hwnd.0.is_null() {
        return false;
    }

    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        let _ = SetForegroundWindow(hwnd);
    }
    true
}

fn close_settings_window() {
    let title = wide(SETTINGS_WINDOW_TITLE);
    let Ok(hwnd) = (unsafe { FindWindowW(PCWSTR(null()), PCWSTR(title.as_ptr())) }) else {
        return;
    };
    if !hwnd.0.is_null() {
        let mut message_result = 0_usize;
        unsafe {
            let _ = SendMessageTimeoutW(
                hwnd,
                WM_CLOSE,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                2_000,
                Some(&mut message_result),
            );
        }
    }
}

pub fn request_exit_all_processes() {
    if !dispatch_notification_action(NotificationAction::ExitAll) {
        exit_all_processes();
    }
}

#[cfg(test)]
mod tray_pixel_tests {
    use super::*;

    #[test]
    fn argb_icon_channels_are_premultiplied() {
        assert_eq!(premultiplied_bgra((126, 64, 253), 255), [253, 64, 126, 255]);
        assert_eq!(premultiplied_bgra((126, 64, 253), 128), [127, 32, 63, 128]);
        assert_eq!(premultiplied_bgra((126, 64, 253), 0), [0, 0, 0, 0]);
    }

    #[test]
    fn notification_xml_escapes_text_and_opens_settings() {
        let xml = notification_xml_with_launch(
            "A < B & C",
            "Quote: \"x\" and 'y'",
            "muteguard://settings",
        );

        assert!(xml.contains(r#"launch="muteguard://settings""#));
        assert!(xml.contains("A &lt; B &amp; C"));
        assert!(xml.contains("Quote: &quot;x&quot; and &apos;y&apos;"));
        assert!(!xml.contains("A < B"));
    }

    #[test]
    fn update_notification_escapes_and_opens_the_release_url() {
        let xml = notification_xml_with_launch(
            "Update available",
            "Download it",
            "https://github.com/Minus193/MuteGuard/releases/latest?from=a&to=b",
        );

        assert!(xml.contains(
            r#"launch="https://github.com/Minus193/MuteGuard/releases/latest?from=a&amp;to=b""#
        ));
    }
}
