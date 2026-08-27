#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiagnosticEntry {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiagnosticSection {
    pub title: String,
    pub entries: Vec<DiagnosticEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiagnosticsSnapshot {
    pub sections: Vec<DiagnosticSection>,
    pub report: String,
}

pub(crate) fn diagnostics_snapshot() -> DiagnosticsSnapshot {
    let (config, config_status) = match load_config() {
        Ok(config) => (config, "Loaded".to_string()),
        Err(error) => (Config::default(), format!("Error: {error:#}")),
    };
    let sections = vec![
        application_diagnostics(&config, config_status),
        windows_diagnostics(),
        audio_diagnostics(&config),
        input_overlay_diagnostics(&config),
    ];
    let report = format_diagnostics_report(&sections);
    DiagnosticsSnapshot { sections, report }
}

fn application_diagnostics(config: &Config, config_status: String) -> DiagnosticSection {
    let background_status = if hidden_window().is_some() {
        "Running"
    } else {
        "Not detected"
    };
    diagnostic_section(
        "Application",
        [
            ("Version", env!("CARGO_PKG_VERSION").to_string()),
            ("Architecture", display_architecture().to_string()),
            ("Background process", background_status.to_string()),
            ("Configuration", config_status),
            (
                "Start with Windows",
                startup_registration_diagnostics(config),
            ),
        ],
    )
}

fn windows_diagnostics() -> DiagnosticSection {
    diagnostic_section(
        "Windows",
        [
            (
                "Build",
                windows_build_number()
                    .map_or_else(|| "Unknown".to_string(), |build| build.to_string()),
            ),
            (
                "WebView2 Runtime",
                availability_label(webview2_runtime_available()),
            ),
            (
                "Mica support",
                availability_label(settings_mica_available()),
            ),
        ],
    )
}

fn audio_diagnostics(config: &Config) -> DiagnosticSection {
    let (audio_status, mute_status, device_id) = match default_capture_diagnostics() {
        Ok((muted, device_id)) => (
            "Available".to_string(),
            if muted { "Muted" } else { "Active" }.to_string(),
            device_id,
        ),
        Err(error) => (
            format!("Unavailable: {error:#}"),
            "Unknown".to_string(),
            "Unavailable".to_string(),
        ),
    };
    diagnostic_section(
        "Audio",
        [
            ("Core Audio", audio_status),
            ("Microphone state", mute_status),
            ("Default endpoint ID", device_id),
            (
                "Device notifications",
                enabled_label(config.device_notifications.notify_changes),
            ),
            (
                "Sound feedback",
                enabled_label(config.sound_feedback.enabled),
            ),
            (
                "Custom mute sound",
                availability_label(custom_sound_available(FeedbackKind::Mute)),
            ),
            (
                "Custom unmute sound",
                availability_label(custom_sound_available(FeedbackKind::Unmute)),
            ),
        ],
    )
}

fn input_overlay_diagnostics(config: &Config) -> DiagnosticSection {
    diagnostic_section(
        "Input and overlay",
        [
            ("Configured hotkeys", config.hotkeys.len().to_string()),
            ("Overlay", enabled_label(config.overlay.enabled)),
            (
                "Configured displays",
                config.overlay.displays.len().to_string(),
            ),
            ("Detected displays", overlay_displays().len().to_string()),
        ],
    )
}

fn default_capture_diagnostics() -> Result<(bool, String)> {
    unsafe {
        let enumerator = audio_device_enumerator()?;
        let device = capture_device(&enumerator)?;
        let device_id = endpoint_device_id(&device)?;
        let volume: IAudioEndpointVolume = device
            .Activate(CLSCTX_ALL, None)
            .context("activate default capture endpoint for diagnostics")?;
        Ok((volume.GetMute()?.as_bool(), device_id))
    }
}

fn startup_registration_diagnostics(config: &Config) -> String {
    let subkey = wide(STARTUP_RUN_SUBKEY);
    let value_name = wide(STARTUP_RUN_VALUE);
    let registered = match read_startup_registration(&subkey, &value_name) {
        Ok(value) => value,
        Err(error) => return format!("Unreadable: {error:#}"),
    };
    if !config.startup.launch_on_startup {
        return if registered.is_none() {
            "Disabled".to_string()
        } else {
            "Disabled, stale Run entry present".to_string()
        };
    }
    let expected = std::env::current_exe()
        .ok()
        .map(|path| startup_command_for_executable(&path));
    if registered == expected {
        "Enabled and registered".to_string()
    } else {
        "Enabled, registration mismatch".to_string()
    }
}

fn diagnostic_section<const N: usize>(
    title: &str,
    entries: [(&str, String); N],
) -> DiagnosticSection {
    DiagnosticSection {
        title: title.to_string(),
        entries: entries
            .into_iter()
            .map(|(label, value)| DiagnosticEntry {
                label: label.to_string(),
                value,
            })
            .collect(),
    }
}

fn enabled_label(enabled: bool) -> String {
    if enabled { "Enabled" } else { "Disabled" }.to_string()
}

fn availability_label(available: bool) -> String {
    if available { "Available" } else { "Unavailable" }.to_string()
}

fn display_architecture() -> &'static str {
    architecture_label(std::env::consts::ARCH)
}

fn architecture_label(architecture: &str) -> &str {
    match architecture {
        "x86_64" => "x64 (AMD64)",
        "x86" => "x86 (32-bit)",
        "aarch64" => "ARM64",
        architecture => architecture,
    }
}

fn format_diagnostics_report(sections: &[DiagnosticSection]) -> String {
    let mut report = format!(
        "MuteGuard diagnostics\nGenerated: Unix {}\n",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs())
    );
    for section in sections {
        report.push_str(&format!("\n[{}]\n", section.title));
        for entry in &section.entries {
            report.push_str(&format!("{}: {}\n", entry.label, entry.value));
        }
    }
    report
}

#[cfg(test)]
mod diagnostics_tests {
    use super::*;

    #[test]
    fn report_has_stable_sections_and_labels() {
        let sections = vec![diagnostic_section(
            "Test",
            [("First", "One".to_string()), ("Second", "Two".to_string())],
        )];
        let report = format_diagnostics_report(&sections);
        assert!(report.contains("[Test]"));
        assert!(report.contains("First: One"));
        assert!(report.contains("Second: Two"));
    }

    #[test]
    fn windows_x64_architecture_has_an_unambiguous_label() {
        assert_eq!(architecture_label("x86_64"), "x64 (AMD64)");
    }
}
