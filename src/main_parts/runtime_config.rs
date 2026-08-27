fn refresh_mute_state() {
    match current_mute_state() {
        Ok(muted) => set_global_mute_state(muted, true),
        Err(error) => {
            report_audio_error("MuteGuard cannot read the microphone state", &error);
            ensure_audio_notification_registration();
        }
    }
}

fn reload_config_now() {
    match load_config() {
        Ok(config) => apply_live_config(&config),
        Err(error) => report_runtime_error(
            "MuteGuard could not reload its settings",
            format!("{error:#}"),
        ),
    }
}

fn load_config() -> Result<Config> {
    let path = config_path()?;
    if path.exists() {
        return parse_config_content(&fs::read_to_string(&path)?);
    }

    let legacy_path = legacy_config_path()?;
    if legacy_path.exists() {
        let config = parse_config_content(&fs::read_to_string(&legacy_path)?)?;
        save_config(&config)?;
        return Ok(config);
    }

    Ok(Config::default())
}

fn parse_config_content(content: &str) -> Result<Config> {
    let raw: Value = serde_json::from_str(content).context("parse MuteGuard configuration")?;
    let mut config: Config =
        serde_json::from_value(raw.clone()).context("decode MuteGuard configuration")?;

    if let Some(raw_hotkeys) = raw.get("hotkeys").and_then(Value::as_array) {
        config.hotkeys = raw_hotkeys
            .iter()
            .filter(|binding| {
                let action_is_toggle = binding
                    .get("action")
                    .and_then(Value::as_str)
                    .is_none_or(|action| action == "ToggleMute");
                let is_gamepad = binding.get("gamepad").is_some_and(|value| !value.is_null());
                action_is_toggle && !is_gamepad
            })
            .filter_map(|binding| serde_json::from_value::<HotkeyBinding>(binding.clone()).ok())
            .collect();
    } else if let Some(shortcut) = raw
        .get("shortcut")
        .and_then(|value| serde_json::from_value::<Shortcut>(value.clone()).ok())
    {
        config.hotkeys = vec![HotkeyBinding {
            shortcut,
            ..HotkeyBinding::default()
        }];
    }

    let startup_has_mute = raw
        .get("startup")
        .and_then(Value::as_object)
        .is_some_and(|startup| startup.contains_key("mute_on_startup"));
    if !startup_has_mute
        && let Some(mute_on_startup) = raw
            .get("auto_mute")
            .and_then(|value| value.get("mute_on_startup"))
            .and_then(Value::as_bool)
    {
        config.startup.mute_on_startup = mute_on_startup;
    }

    normalize_hotkeys(&mut config.hotkeys);
    normalize_appearance_config(&mut config.appearance);
    normalize_overlay_config(&mut config.overlay);
    normalize_tray_icon_config(&mut config.tray_icon);
    Ok(config)
}

fn save_config(config: &Config) -> Result<()> {
    let directory = app_config_dir()?;
    fs::create_dir_all(&directory).context("create MuteGuard configuration directory")?;
    let path = directory.join("config.json");
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary_path = directory.join(format!(
        "config.json.{}.{nonce}.tmp",
        std::process::id()
    ));
    let serialized =
        serde_json::to_string_pretty(config).context("serialize MuteGuard configuration")?;
    fs::write(&temporary_path, serialized).context("write temporary MuteGuard configuration")?;
    let temporary_wide = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut replace_error = None;
    for retry_delay_ms in [0, 25, 75, 150, 300] {
        if retry_delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(retry_delay_ms));
        }
        match unsafe {
            MoveFileExW(
                PCWSTR(temporary_wide.as_ptr()),
                PCWSTR(path_wide.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } {
            Ok(()) => {
                replace_error = None;
                break;
            }
            Err(error) => replace_error = Some(error),
        }
    }
    if let Some(error) = replace_error {
        let _ = fs::remove_file(&temporary_path);
        return Err(error).context("atomically replace MuteGuard configuration");
    }

    if let Some(hwnd) = hidden_window() {
        unsafe {
            let _ = PostMessageW(hwnd, WM_CONFIG_CHANGED, WPARAM(0), LPARAM(0));
        }
    }
    Ok(())
}

pub(crate) fn reset_config_to_defaults() -> Result<(Config, Option<PathBuf>, Option<String>)> {
    let path = config_path()?;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let backup_path = path.with_file_name(format!(
        "config.invalid-{timestamp}-{}.json",
        std::process::id()
    ));
    let invalid_source = if path.exists() {
        Some(path)
    } else {
        let legacy_path = legacy_config_path()?;
        legacy_path.exists().then_some(legacy_path)
    };

    let backup_path = back_up_invalid_config(invalid_source.as_deref(), &backup_path)?;

    let config = Config::default();
    save_config(&config)?;
    let startup_warning = sync_startup_registration(false)
        .err()
        .map(|error| format!("Windows startup registration could not be removed: {error:#}"));
    Ok((config, backup_path, startup_warning))
}

fn back_up_invalid_config(
    invalid_source: Option<&Path>,
    backup_path: &Path,
) -> Result<Option<PathBuf>> {
    if let Some(invalid_source) = invalid_source {
        if let Some(directory) = backup_path.parent() {
            fs::create_dir_all(directory)
                .context("create MuteGuard configuration backup directory")?;
        }
        fs::copy(invalid_source, backup_path).with_context(|| {
            format!(
                "back up the invalid configuration from {} to {}",
                invalid_source.display(),
                backup_path.display()
            )
        })?;
        Ok(Some(backup_path.to_path_buf()))
    } else {
        Ok(None)
    }
}

pub(crate) fn normalize_tray_icon_config(tray_icon: &mut TrayIconConfig) {
    if !matches!(tray_icon.variant.as_str(), "Logo" | "StatusMic" | "ColorDot") {
        tray_icon.variant = default_tray_icon_variant();
    }
    if tray_icon.status_style == "Colored" {
        tray_icon.status_style = "Custom".to_string();
    } else if !matches!(
        tray_icon.status_style.as_str(),
        "Custom" | "Monochrome" | "SystemColor"
    ) {
        tray_icon.status_style = default_tray_icon_status_style();
    }
    tray_icon.status_color = parse_hex_color(&tray_icon.status_color).map_or_else(
        default_tray_icon_status_color,
        |(red, green, blue)| format!("#{red:02x}{green:02x}{blue:02x}"),
    );
    tray_icon.icon_pair = crate::overlay_icons::overlay_icon_pair(&tray_icon.icon_pair)
        .id
        .to_string();
}

pub(crate) fn normalize_appearance_config(appearance: &mut AppearanceSettings) {
    if !matches!(appearance.accent_style.as_str(), "SystemColor" | "Custom") {
        appearance.accent_style = default_app_accent_style();
    }
    appearance.accent_color = parse_hex_color(&appearance.accent_color).map_or_else(
        default_app_accent_color,
        |(red, green, blue)| format!("#{red:02x}{green:02x}{blue:02x}"),
    );
}

fn normalize_overlay_config(overlay: &mut OverlayConfig) {
    if overlay.variant == "MicIcon" && overlay.show_text {
        overlay.variant = "IconText".to_string();
    }
    if !matches!(
        overlay.visibility.as_str(),
        "Always" | "WhenMuted" | "WhenUnmuted" | "AfterToggle"
    ) {
        overlay.visibility = default_overlay_visibility();
    }
    if !matches!(
        overlay.variant.as_str(),
        "MicIcon" | "IconText" | "Text" | "Dot"
    ) {
        overlay.variant = default_overlay_variant();
    }
    if overlay.icon_style == "Colored" {
        overlay.icon_style = "Custom".to_string();
    } else if !matches!(
        overlay.icon_style.as_str(),
        "Monochrome" | "SystemColor" | "Custom"
    ) {
        overlay.icon_style = default_overlay_icon_style();
    }
    overlay.icon_color = parse_hex_color(&overlay.icon_color).map_or_else(
        default_overlay_icon_color,
        |(red, green, blue)| format!("#{red:02x}{green:02x}{blue:02x}"),
    );
    overlay.border_color = parse_hex_color(&overlay.border_color).map_or_else(
        default_overlay_border_color,
        |(red, green, blue)| format!("#{red:02x}{green:02x}{blue:02x}"),
    );
    if !matches!(
        overlay.background_style.as_str(),
        "Dark" | "Light" | "Transparent"
    ) {
        overlay.background_style = default_overlay_background_style();
    }

    overlay.icon_pair = crate::overlay_icons::overlay_icon_pair(&overlay.icon_pair)
        .id
        .to_string();
    overlay.position_x = overlay.position_x.clamp(0.0, 100.0);
    overlay.position_y = overlay.position_y.clamp(0.0, 100.0);
    overlay.duration_secs = overlay.duration_secs.clamp(0.5, 10.0);
    overlay.scale = overlay.scale.clamp(10, 400);
    overlay.text_font_weight = overlay.text_font_weight.clamp(100, 900);
    overlay.background_opacity = overlay.background_opacity.min(100);
    overlay.content_opacity = overlay.content_opacity.clamp(20, 100);
    overlay.border_radius = overlay.border_radius.min(24);
    overlay.show_text = false;

    overlay.display = normalize_overlay_display_id(&overlay.display);
    if overlay.displays.is_empty() {
        overlay.displays.push(overlay.display.clone());
    } else {
        overlay.displays = overlay
            .displays
            .iter()
            .map(|display| normalize_overlay_display_id(display))
            .fold(Vec::<String>::new(), |mut displays, display| {
                if !displays.contains(&display) {
                    displays.push(display);
                }
                displays
            });
    }
    overlay.display = overlay.displays[0].clone();
    if overlay.muted_label.trim().is_empty() {
        overlay.muted_label = default_overlay_muted_label();
    }
    if overlay.unmuted_label.trim().is_empty() {
        overlay.unmuted_label = default_overlay_unmuted_label();
    }
    overlay.text_font = normalize_overlay_font_family(&overlay.text_font);
}

fn normalize_overlay_display_id(display: &str) -> String {
    if display.trim().is_empty() {
        return OVERLAY_DISPLAY_PRIMARY.to_string();
    }
    if let Some(index) = display
        .strip_prefix("Monitor")
        .and_then(|index| index.parse::<usize>().ok())
    {
        return overlay_displays().get(index.saturating_sub(1)).map_or_else(
            || OVERLAY_DISPLAY_PRIMARY.to_string(),
            |display| display.id.clone(),
        );
    }
    display.to_string()
}

pub(crate) fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let digits = value.trim().strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    let color = u32::from_str_radix(digits, 16).ok()?;
    Some((
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
    ))
}

pub(crate) fn apply_live_config(config: &Config) {
    let mut state = STATE.lock().unwrap();
    state.hotkeys.clone_from(&config.hotkeys);
    state.mute_on_startup = config.startup.mute_on_startup;
    if !state.mute_on_startup {
        state.startup_mute_pending = false;
    }
    state.overlay = config.overlay.clone();
    state.tray_icon = config.tray_icon.clone();
    state.hotkeys_down.clear();
    state.keyboard_keys_down.clear();
    state.mouse_buttons_down.clear();
    drop(state);

    refresh_tray_icon();
    apply_overlay_visibility();
}

fn normalize_hotkeys(hotkeys: &mut [HotkeyBinding]) {
    let mut seen = HashSet::new();
    for hotkey in hotkeys {
        if hotkey.id.trim().is_empty() || !seen.insert(hotkey.id.clone()) {
            loop {
                hotkey.id = default_hotkey_id();
                if seen.insert(hotkey.id.clone()) {
                    break;
                }
            }
        }
        hotkey.shortcut = hotkey.shortcut.clone().normalized();
        hotkey.target = match hotkey.target.as_deref() {
            Some(HOTKEY_TARGET_ALL_MICROPHONES) => {
                Some(HOTKEY_TARGET_ALL_MICROPHONES.to_string())
            }
            _ => None,
        };
    }
}

fn config_path() -> Result<PathBuf> {
    Ok(app_config_dir()?.join("config.json"))
}

fn legacy_config_path() -> Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA").context("APPDATA is not set")?;
    Ok(PathBuf::from(appdata).join("SilenceV2").join("config.json"))
}

fn app_config_dir() -> Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA").context("APPDATA is not set")?;
    Ok(PathBuf::from(appdata).join("MuteGuard"))
}

pub(crate) fn sync_startup_registration(enabled: bool) -> Result<()> {
    let subkey = wide(STARTUP_RUN_SUBKEY);
    let value_name = wide(STARTUP_RUN_VALUE);
    let registered_command = read_startup_registration(&subkey, &value_name)?;

    if enabled {
        let executable = std::env::current_exe()
            .context("locate MuteGuard executable for startup registration")?;
        let command = startup_command_for_executable(&executable);
        if registered_command.as_deref() == Some(command.as_str()) {
            return Ok(());
        }
        let command_wide = wide(&command);
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                PCWSTR(value_name.as_ptr()),
                REG_SZ.0,
                Some(command_wide.as_ptr() as *const c_void),
                (command_wide.len() * size_of::<u16>()) as u32,
            )
        };
        anyhow::ensure!(
            status == ERROR_SUCCESS,
            "startup registration failed with status {status:?}"
        );
        return Ok(());
    }

    if registered_command.is_none() {
        return Ok(());
    }

    let status = unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
        )
    };
    anyhow::ensure!(
        status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND,
        "startup registration removal failed with status {status:?}"
    );
    Ok(())
}

fn read_startup_registration(subkey: &[u16], value_name: &[u16]) -> Result<Option<String>> {
    let mut buffer = vec![0_u16; 32_768];
    let mut data_size = (buffer.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut data_size),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    anyhow::ensure!(
        status == ERROR_SUCCESS,
        "read startup registration failed with status {status:?}"
    );
    let character_count = (data_size as usize / size_of::<u16>()).min(buffer.len());
    Ok(Some(wide_buf_to_string(&buffer[..character_count])))
}

fn startup_command_for_executable(executable: &Path) -> String {
    format!("\"{}\"", executable.display())
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn custom_overlay_color_is_normalized_and_invalid_values_are_replaced() {
        let mut custom = OverlayConfig {
            icon_style: "Custom".to_string(),
            icon_color: " #A1b2C3 ".to_string(),
            ..OverlayConfig::default()
        };
        normalize_overlay_config(&mut custom);
        assert_eq!(custom.icon_color, "#a1b2c3");

        custom.icon_color = "#7D42FB".to_string();
        normalize_overlay_config(&mut custom);
        assert_eq!(custom.icon_color, "#7d42fb");

        custom.icon_color = "red".to_string();
        normalize_overlay_config(&mut custom);
        assert_eq!(custom.icon_color, default_overlay_icon_color());
    }

    #[test]
    fn custom_overlay_border_color_is_normalized_and_invalid_values_are_replaced() {
        let mut overlay = OverlayConfig {
            border_color: " #A1b2C3 ".to_string(),
            ..OverlayConfig::default()
        };
        normalize_overlay_config(&mut overlay);
        assert_eq!(overlay.border_color, "#a1b2c3");

        overlay.border_color = "not-a-color".to_string();
        normalize_overlay_config(&mut overlay);
        assert_eq!(overlay.border_color, default_overlay_border_color());
    }

    #[test]
    fn startup_command_quotes_paths_with_spaces() {
        assert_eq!(
            startup_command_for_executable(Path::new(r"C:\Program Files\MuteGuard\muteguard.exe")),
            r#""C:\Program Files\MuteGuard\muteguard.exe""#
        );
    }

    #[test]
    fn custom_application_accent_is_normalized() {
        let mut appearance = AppearanceSettings {
            accent_style: "Custom".to_string(),
            accent_color: "#7D42FB".to_string(),
        };
        normalize_appearance_config(&mut appearance);
        assert_eq!(appearance.accent_color, "#7d42fb");
        assert_eq!(effective_app_accent_css(&Config {
            appearance,
            ..Config::default()
        }), "rgb(125, 66, 251)");
    }

    #[test]
    fn legacy_colored_styles_migrate_to_the_editable_colored_mode() {
        let mut overlay = OverlayConfig {
            icon_style: "Colored".to_string(),
            ..OverlayConfig::default()
        };
        normalize_overlay_config(&mut overlay);
        assert_eq!(overlay.icon_style, "Custom");

        let mut tray = TrayIconConfig {
            status_style: "Colored".to_string(),
            status_color: " #12AbEF ".to_string(),
            ..TrayIconConfig::default()
        };
        normalize_tray_icon_config(&mut tray);
        assert_eq!(tray.status_style, "Custom");
        assert_eq!(tray.status_color, "#12abef");

        tray.status_color = "not-a-color".to_string();
        normalize_tray_icon_config(&mut tray);
        assert_eq!(tray.status_color, default_tray_icon_status_color());
    }

    #[test]
    fn legacy_overlay_display_migrates_to_a_unique_multiselect() {
        let mut overlay = OverlayConfig {
            display: "Monitor1".to_string(),
            displays: Vec::new(),
            ..OverlayConfig::default()
        };
        normalize_overlay_config(&mut overlay);
        assert_eq!(overlay.displays, [overlay.display.clone()]);

        overlay.displays = vec![
            OVERLAY_DISPLAY_PRIMARY.to_string(),
            OVERLAY_DISPLAY_PRIMARY.to_string(),
        ];
        normalize_overlay_config(&mut overlay);
        assert_eq!(overlay.displays, [OVERLAY_DISPLAY_PRIMARY]);
        assert_eq!(overlay.display, OVERLAY_DISPLAY_PRIMARY);
    }

    #[test]
    fn new_overlay_defaults_to_the_top_center_anchor() {
        let overlay = OverlayConfig::default();
        assert_eq!(overlay.position_x, 50.0);
        assert_eq!(overlay.position_y, 0.0);
    }

    #[test]
    fn styled_segoe_faces_use_the_weight_control_instead() {
        let mut overlay = OverlayConfig {
            text_font: "Segoe UI Light".to_string(),
            text_font_weight: 400,
            ..OverlayConfig::default()
        };
        normalize_overlay_config(&mut overlay);
        assert_eq!(overlay.text_font, "Segoe UI");
        assert_eq!(overlay.text_font_weight, 400);
    }

    #[test]
    fn legacy_config_keeps_only_supported_toggle_bindings() {
        let config = parse_config_content(
            r#"{
                "hotkeys": [
                    {
                        "id": "all",
                        "action": "ToggleMute",
                        "shortcut": {"ctrl": true, "alt": false, "shift": false, "win": false, "vk": 77},
                        "target": "__all_microphones__"
                    },
                    {
                        "id": "force",
                        "action": "Mute",
                        "shortcut": {"ctrl": false, "alt": true, "shift": false, "win": false, "vk": 77}
                    },
                    {
                        "id": "pad",
                        "action": "ToggleMute",
                        "gamepad": {"inputs": [{"button": "South"}]}
                    }
                ],
                "startup": {"launch_on_startup": false},
                "auto_mute": {"mute_on_startup": true},
                "overlay": {"visibility": "WhenMicInUse", "position_x": 140.0},
                "tray_icon": {"variant": "Legacy"}
            }"#,
        )
        .expect("legacy configuration should migrate");

        assert_eq!(config.hotkeys.len(), 1);
        assert_eq!(config.hotkeys[0].id, "all");
        assert_eq!(
            config.hotkeys[0].target.as_deref(),
            Some(HOTKEY_TARGET_ALL_MICROPHONES)
        );
        assert!(config.startup.mute_on_startup);
        assert_eq!(config.overlay.visibility, "WhenMuted");
        assert_eq!(config.overlay.position_x, 100.0);
        assert_eq!(config.tray_icon.variant, "StatusMic");
    }

    #[test]
    fn normalization_removes_device_specific_targets_and_duplicate_ids() {
        let mut hotkeys = vec![
            HotkeyBinding {
                id: "duplicate".to_string(),
                target: Some("legacy-device-id".to_string()),
                ..HotkeyBinding::default()
            },
            HotkeyBinding {
                id: "duplicate".to_string(),
                target: Some(HOTKEY_TARGET_ALL_MICROPHONES.to_string()),
                ..HotkeyBinding::default()
            },
        ];

        normalize_hotkeys(&mut hotkeys);

        assert_ne!(hotkeys[0].id, hotkeys[1].id);
        assert_eq!(hotkeys[0].target, None);
        assert_eq!(
            hotkeys[1].target.as_deref(),
            Some(HOTKEY_TARGET_ALL_MICROPHONES)
        );
        assert_eq!(
            hotkeys[0].shortcut.keyboard_keys(),
            vec![VK_CONTROL, VK_MENU, b'M' as u32]
        );
        assert_eq!(hotkeys[0].shortcut.vk, 0);
        assert!(!hotkeys[0].shortcut.ctrl);
    }

    #[test]
    fn generated_hotkey_ids_are_unique_in_a_burst() {
        let ids = (0..10_000)
            .map(|_| default_hotkey_id())
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), 10_000);
    }

    #[test]
    fn recovery_backs_up_a_legacy_invalid_configuration() {
        let test_root = std::env::temp_dir().join(default_hotkey_id());
        let legacy_path = test_root.join("SilenceV2").join("config.json");
        let backup_path = test_root
            .join("MuteGuard")
            .join("config.invalid-test.json");
        fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        fs::write(&legacy_path, "{invalid-json").unwrap();

        let result = back_up_invalid_config(Some(&legacy_path), &backup_path).unwrap();

        assert_eq!(result.as_deref(), Some(backup_path.as_path()));
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "{invalid-json");
        fs::remove_dir_all(&test_root).unwrap();
    }

    #[test]
    fn empty_shortcut_is_inert_and_has_a_clear_label() {
        let shortcut = Shortcut::empty();
        assert_eq!(shortcut.display(), "None");
        assert!(!shortcut.is_pressed(
            b'M' as u32,
            false,
            &HashSet::new(),
            &HashSet::new(),
        ));
        assert!(!shortcut.is_pressed(
            VK_CONTROL,
            true,
            &HashSet::from([VK_CONTROL]),
            &HashSet::new(),
        ));
    }

    #[test]
    fn keyboard_shortcuts_respect_exact_and_ignored_modifiers() {
        let shortcut = Shortcut {
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
            vk: b'M' as u32,
            keyboard_keys: Vec::new(),
            mouse_buttons: Vec::new(),
        };
        let ctrl = HashSet::from([VK_CONTROL, b'M' as u32]);
        let ctrl_shift = HashSet::from([VK_CONTROL, VK_SHIFT, b'M' as u32]);

        assert!(shortcut.is_pressed(b'M' as u32, false, &ctrl, &HashSet::new()));
        assert!(!shortcut.is_pressed(b'M' as u32, false, &ctrl_shift, &HashSet::new()));
        assert!(shortcut.is_pressed(b'M' as u32, true, &ctrl_shift, &HashSet::new()));
        assert!(!shortcut.is_pressed(b'N' as u32, true, &ctrl_shift, &HashSet::new()));
    }

    #[test]
    fn mouse_chords_require_every_configured_button() {
        let shortcut = Shortcut {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: 0,
            keyboard_keys: Vec::new(),
            mouse_buttons: vec![VK_LBUTTON, VK_RBUTTON],
        };
        let one_button = HashSet::from([VK_LBUTTON]);
        let both_buttons = HashSet::from([VK_LBUTTON, VK_RBUTTON]);

        assert!(!shortcut.is_pressed(
            VK_LBUTTON,
            false,
            &HashSet::new(),
            &one_button,
        ));
        assert!(shortcut.is_pressed(
            VK_RBUTTON,
            false,
            &HashSet::new(),
            &both_buttons,
        ));
    }

    #[test]
    fn arbitrary_keyboard_chords_require_every_recorded_key() {
        let shortcut = Shortcut::from_inputs(vec![b'A' as u32, b'B' as u32], Vec::new());
        let one_key = HashSet::from([b'A' as u32]);
        let both_keys = HashSet::from([b'A' as u32, b'B' as u32]);

        assert!(!shortcut.is_pressed(b'A' as u32, false, &one_key, &HashSet::new()));
        assert!(shortcut.is_pressed(b'B' as u32, false, &both_keys, &HashSet::new()));
    }

    #[test]
    fn new_shortcuts_serialize_only_the_multi_key_representation() {
        let value = serde_json::to_value(Shortcut::default()).unwrap();

        assert_eq!(
            value["keyboard_keys"],
            serde_json::json!([VK_CONTROL, VK_MENU, b'M' as u32])
        );
        assert!(value.get("ctrl").is_none());
        assert!(value.get("alt").is_none());
        assert!(value.get("vk").is_none());
    }
}
