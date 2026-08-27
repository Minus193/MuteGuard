#[cfg(target_os = "windows")]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;
use dioxus::prelude::*;

mod controls;
mod sections;
mod tabs;

use tabs::SettingsTab;

const APP_IMAGE: Asset = asset!("/assets/muteguard.png");
pub(crate) const APP_ICO: Asset = asset!("/assets/muteguard.ico");
const BRICOLAGE_GROTESQUE_FONT: Asset = asset!("/assets/fonts/BricolageGrotesque-latin.woff2");
const PLUS_JAKARTA_SANS_FONT: Asset = asset!("/assets/fonts/PlusJakartaSans-latin.woff2");
const INTER_FONT: Asset = asset!("/assets/fonts/InterVariable.woff2");
const SETTINGS_LAYOUT_SCRIPT: &str = include_str!("../../assets/scripts/settings-layout.js");
const GLOBAL_CSS: Asset = asset!("/assets/styles/global.css", AssetOptions::css());
const CONTROLS_CSS: Asset = asset!("/assets/styles/controls.css", AssetOptions::css());
const LAYOUT_CSS: Asset = asset!("/assets/styles/layout.css", AssetOptions::css());
const TITLEBAR_CSS: Asset = asset!("/assets/styles/titlebar.css", AssetOptions::css());
const TABS_CSS: Asset = asset!("/assets/styles/tabs.css", AssetOptions::css());
const GENERAL_CSS: Asset = asset!("/assets/styles/general.css", AssetOptions::css());
const OVERLAY_CSS: Asset = asset!("/assets/styles/overlay.css", AssetOptions::css());
const HOTKEYS_CSS: Asset = asset!("/assets/styles/hotkeys.css", AssetOptions::css());
const GUIDE_CSS: Asset = asset!("/assets/styles/guide.css", AssetOptions::css());

const CLOSE_ICON: Asset = asset!("/assets/icons/codicon_close.svg");
const SETTINGS_ICON: Asset = asset!("/assets/icons/codicon_settings-gear.svg");
const SETTINGS_LINEAR_ICON: Asset = asset!("/assets/icons/settings-linear.svg");
const KEYBOARD_LINEAR_ICON: Asset = asset!("/assets/icons/keyboard-linear.svg");
const MONITOR_LINEAR_ICON: Asset = asset!("/assets/icons/monitor-linear.svg");
const WIDGET_LINEAR_ICON: Asset = asset!("/assets/icons/widget-linear.svg");
const PLUS_LINEAR_ICON: Asset = asset!("/assets/icons/plus-linear.svg");
const TRASH_ICON: Asset = asset!("/assets/icons/trash-bin-trash-linear.svg");
const CHEVRON_ICON: Asset = asset!("/assets/icons/alt-arrow-down-linear.svg");
const MIC_ICON: Asset = asset!("/assets/icons/mic.svg");
const MIC_OFF_ICON: Asset = asset!("/assets/icons/mic-off.svg");
const CLOCK_ICON: Asset = asset!("/assets/icons/clock-circle-linear.svg");
const PALETTE_ICON: Asset = asset!("/assets/icons/pallete-2-linear.svg");
const CONTRAST_ICON: Asset = asset!("/assets/icons/ic-baseline-contrast.svg");
const MOON_ICON: Asset = asset!("/assets/icons/moon-linear.svg");
const SUN_ICON: Asset = asset!("/assets/icons/sun-2-linear.svg");
const RECORD_ICON: Asset = asset!("/assets/icons/record-circle-linear.svg");
const SPEAKER_ICON: Asset = asset!("/assets/icons/speaker-linear.svg");
const DIAGNOSTICS_ICON: Asset = asset!("/assets/icons/diagnostics-linear.svg");
const HELP_ICON: Asset = asset!("/assets/icons/help-circle-linear.svg");

#[derive(Clone, PartialEq)]
pub(crate) struct SettingsSnapshot {
    pub(crate) config: crate::Config,
    pub(crate) overlay_displays: Option<Vec<crate::OverlayDisplay>>,
    pub(crate) system_fonts: Option<Vec<crate::SystemFont>>,
    pub(crate) muted: bool,
    pub(crate) error: Option<String>,
    pub(crate) notice: Option<String>,
    pub(crate) config_recovery_available: bool,
}

impl SettingsSnapshot {
    fn load() -> Self {
        let (config, config_error, config_recovery_available) = match crate::load_config() {
            Ok(config) => (config, None, false),
            Err(error) => (
                crate::Config::default(),
                Some(format!(
                    "MuteGuard could not load config.json: {error:#}. Your current file can be backed up and replaced with safe defaults."
                )),
                true,
            ),
        };
        let (muted, audio_error) = match crate::mic_mute_state() {
            Ok(muted) => (muted, None),
            Err(error) => (
                false,
                Some(format!(
                    "MuteGuard cannot read the microphone state: {error:#}"
                )),
            ),
        };
        Self {
            config,
            overlay_displays: None,
            system_fonts: None,
            muted,
            error: config_error.or(audio_error),
            notice: None,
            config_recovery_available,
        }
    }
}

pub(crate) fn update_settings(
    mut settings: Signal<SettingsSnapshot>,
    update: impl FnOnce(&mut crate::Config),
) -> bool {
    let mut config = settings.peek().config.clone();
    let startup_was_enabled = config.startup.launch_on_startup;
    update(&mut config);
    crate::normalize_hotkeys(&mut config.hotkeys);
    crate::normalize_appearance_config(&mut config.appearance);
    crate::normalize_overlay_config(&mut config.overlay);
    crate::normalize_tray_icon_config(&mut config.tray_icon);
    crate::normalize_sound_feedback_config(&mut config.sound_feedback);
    let startup_changed = config.startup.launch_on_startup != startup_was_enabled;
    if startup_changed
        && let Err(error) = crate::sync_startup_registration(config.startup.launch_on_startup)
    {
        set_settings_error(
            settings,
            format!("Could not update Windows startup: {error:#}"),
        );
        return false;
    }
    if let Err(error) = crate::save_config(&config) {
        if startup_changed {
            let _ = crate::sync_startup_registration(startup_was_enabled);
        }
        set_settings_error(settings, format!("Could not save settings: {error:#}"));
        return false;
    }

    let current = settings.peek().clone();
    settings.set(SettingsSnapshot {
        config,
        overlay_displays: current.overlay_displays,
        system_fonts: current.system_fonts,
        muted: current.muted,
        error: None,
        notice: None,
        config_recovery_available: false,
    });
    true
}

fn set_settings_error(mut settings: Signal<SettingsSnapshot>, message: String) {
    let mut current = settings.peek().clone();
    current.error = Some(message);
    settings.set(current);
}

pub(crate) fn set_settings_notice(mut settings: Signal<SettingsSnapshot>, message: String) {
    let mut current = settings.peek().clone();
    current.error = None;
    current.notice = Some(message);
    settings.set(current);
}

fn recover_default_settings(mut settings: Signal<SettingsSnapshot>) {
    match crate::reset_config_to_defaults() {
        Ok((config, backup_path, startup_warning)) => {
            let mut notice = backup_path.map_or_else(
                || "Settings were reset to safe defaults.".to_string(),
                |backup_path| {
                    format!(
                        "Settings were reset. The invalid file was preserved as {}.",
                        backup_path.display()
                    )
                },
            );
            if let Some(startup_warning) = startup_warning {
                notice.push_str(&format!(" {startup_warning}"));
            }
            let mut recovered = SettingsSnapshot::load();
            recovered.config = config;
            recovered.notice = Some(notice);
            settings.set(recovered);
        }
        Err(error) => set_settings_error(
            settings,
            format!("Could not back up and reset settings: {error:#}"),
        ),
    }
}

pub(crate) fn settings_startup_head() -> String {
    let theme_style = crate::WindowsAccent::load().css_vars();
    format!(
        r#"<link rel="icon" href="{APP_ICO}" type="image/x-icon">
<style>
html, body, #main, #root {{ margin: 0; width: 100%; height: 100%; overflow: hidden; background: transparent !important; color: rgb(251, 251, 251); }}
#main, .window {{ width: 100vw; height: 100vh; overflow: hidden; }}
button:focus-visible, input:focus-visible, select:focus-visible {{ outline: 2px solid var(--windows-accent); outline-offset: 2px; }}
</style>
<style>{}</style>
<link rel="stylesheet" href="{GLOBAL_CSS}">
<link rel="stylesheet" href="{CONTROLS_CSS}">
<link rel="stylesheet" href="{LAYOUT_CSS}">
<link rel="stylesheet" href="{TITLEBAR_CSS}">
<link rel="stylesheet" href="{TABS_CSS}">
<link rel="stylesheet" href="{GENERAL_CSS}">
<link rel="stylesheet" href="{OVERLAY_CSS}">
<link rel="stylesheet" href="{HOTKEYS_CSS}">
<link rel="stylesheet" href="{GUIDE_CSS}">
<style>{theme_style}</style>
<style>{}</style>
<script>{}</script>
<script>
window.addEventListener('keydown', (event) => {{
  if (document.querySelector('.shortcut-display.recording')) event.preventDefault();
}}, true);
</script>"#,
        settings_font_face(),
        settings_icon_style(),
        SETTINGS_LAYOUT_SCRIPT,
    )
}

fn settings_font_face() -> String {
    format!(
        r#"@font-face {{ font-family: "Bricolage Grotesque"; src: url("{BRICOLAGE_GROTESQUE_FONT}") format("woff2"); font-weight: 400 800; font-display: swap; }}
@font-face {{ font-family: "Plus Jakarta Sans"; src: url("{PLUS_JAKARTA_SANS_FONT}") format("woff2"); font-weight: 400 600; font-display: swap; }}
@font-face {{ font-family: "Inter"; src: url("{INTER_FONT}") format("woff2"); font-weight: 100 900; font-display: swap; }}"#
    )
}

fn settings_icon_style() -> String {
    format!(
        r#".titlebar-settings {{ --titlebar-icon: url("{SETTINGS_ICON}"); }}
.titlebar-close {{ --titlebar-icon: url("{CLOSE_ICON}"); }}
.icon-settings {{ --icon: url("{SETTINGS_LINEAR_ICON}"); }}
.icon-keyboard {{ --icon: url("{KEYBOARD_LINEAR_ICON}"); }}
.icon-monitor {{ --icon: url("{MONITOR_LINEAR_ICON}"); }}
.icon-widget {{ --icon: url("{WIDGET_LINEAR_ICON}"); }}
.icon-plus {{ --icon: url("{PLUS_LINEAR_ICON}"); }}
.icon-trash {{ --icon: url("{TRASH_ICON}"); }}
.icon-chevron-down {{ --icon: url("{CHEVRON_ICON}"); }}
.icon-mic, .icon-mic-lucide {{ --icon: url("{MIC_ICON}"); }}
.icon-mic-muted {{ --icon: url("{MIC_OFF_ICON}"); }}
.icon-clock-circle {{ --icon: url("{CLOCK_ICON}"); }}
.icon-palette {{ --icon: url("{PALETTE_ICON}"); }}
.icon-contrast {{ --icon: url("{CONTRAST_ICON}"); }}
.icon-moon {{ --icon: url("{MOON_ICON}"); }}
.icon-sun {{ --icon: url("{SUN_ICON}"); }}
.icon-record {{ --icon: url("{RECORD_ICON}"); }}
.icon-speaker {{ --icon: url("{SPEAKER_ICON}"); }}
.icon-diagnostics {{ --icon: url("{DIAGNOSTICS_ICON}"); }}
.icon-help {{ --icon: url("{HELP_ICON}"); }}"#
    )
}

pub(crate) fn settings_app() -> Element {
    let desktop = dioxus::desktop::use_window();
    #[cfg(target_os = "windows")]
    use_hook({
        let desktop = desktop.clone();
        move || {
            crate::install_settings_window_guard(desktop.hwnd());
            desktop.set_visible(true);
            desktop.set_focus();
        }
    });

    let drag_desktop = desktop.clone();
    let close_desktop = desktop;
    let settings = use_signal(SettingsSnapshot::load);
    let active_tab = use_signal(|| SettingsTab::General);
    let mut open_select = use_signal(|| None::<String>);
    use_context_provider(|| open_select);
    let accent_rule = format!(
        ":root {{ --windows-accent: {}; }}",
        crate::effective_app_accent_css(&settings().config)
    );

    rsx! {
        div {
            class: if crate::effective_settings_mica_enabled(&settings().config) { "window mica-enabled" } else { "window" },
            style { "{accent_rule}" }
            div {
                class: "titlebar",
                onmousedown: move |_| drag_desktop.drag(),
                div { class: "titlebar-brand",
                    img { class: "titlebar-brand-icon", src: APP_IMAGE, alt: "MuteGuard" }
                    span { class: "titlebar-brand-name", "MuteGuard" }
                }
                div { class: "title-spacer" }
                button {
                    class: "titlebar-button",
                    id: "close",
                    aria_label: "Close Settings",
                    onmousedown: move |event| event.stop_propagation(),
                    onclick: move |_| {
                        crate::set_settings_hotkey_recording(false);
                        close_desktop.set_visible(false);
                        close_desktop.close();
                    },
                    span { class: "titlebar-glyph titlebar-close" }
                }
            }

            div { class: "body",
                {tabs::render(active_tab)}
                main { class: "content",
                    if let Some(error) = settings().error {
                        div {
                            class: "settings-error",
                            role: "alert",
                            aria_live: "assertive",
                            div { class: "settings-message-copy", "{error}" }
                            if settings().config_recovery_available {
                                button {
                                    r#type: "button",
                                    class: "settings-message-action",
                                    onclick: move |_| recover_default_settings(settings),
                                    "Back up and reset"
                                }
                            }
                        }
                    } else if let Some(notice) = settings().notice {
                        div {
                            class: "settings-notice",
                            role: "status",
                            aria_live: "polite",
                            "{notice}"
                        }
                    }
                    div {
                        class: "content-scroll",
                        onscroll: move |_| open_select.set(None),
                        div { class: "content-inner",
                            {sections::render(active_tab(), settings)}
                        }
                    }
                }
            }
        }
    }
}
