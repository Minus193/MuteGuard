use dioxus::prelude::*;

use crate::gui::controls::{ColorPicker, Select, SelectOption};

pub(crate) fn render(settings: Signal<super::super::SettingsSnapshot>) -> Element {
    let snapshot = settings();
    let accent_options = vec![
        SelectOption::new("SystemColor", crate::gui::SYSTEM_COLOR_LABEL).icon("icon-widget"),
        SelectOption::new("Custom", "Custom color").icon("icon-palette"),
    ];

    rsx! {
        section { class: "general-panel", id: "general-settings",
            div { class: "general-header",
                h1 { "General" }
                p { "Startup, notifications, updates, and application appearance." }
            }

            div { class: "settings-card-grid",
                section { class: "sound-card startup-card",
                    div { class: "sound-card-title startup-row",
                        div { class: "startup-copy",
                            h2 { "Start with Windows" }
                            p { "Launch MuteGuard silently when you sign in." }
                        }
                        Toggle {
                            checked: snapshot.config.startup.launch_on_startup,
                            onchange: move |checked| {
                                super::super::update_settings(settings, |config| {
                                    config.startup.launch_on_startup = checked;
                                });
                            }
                        }
                    }
                }

                section { class: "sound-card startup-card",
                    div { class: "sound-card-title startup-row",
                        div { class: "startup-copy",
                            h2 { "Mute microphone on startup" }
                            p { "Apply mute once, after the default communications microphone is ready." }
                        }
                        Toggle {
                            checked: snapshot.config.startup.mute_on_startup,
                            onchange: move |checked| {
                                super::super::update_settings(settings, |config| {
                                    config.startup.mute_on_startup = checked;
                                });
                            }
                        }
                    }
                }

                DeviceNotificationCard {
                    settings,
                    enabled: snapshot.config.device_notifications.notify_changes,
                }

                UpdateCard {
                    settings,
                    enabled: snapshot.config.updates.check_automatically,
                }

                section { class: "sound-card appearance-card",
                    div { class: "sound-card-title",
                        div { class: "startup-copy",
                            h2 { "Application accent" }
                            p { "Use the Windows accent or choose a color for controls, focus and highlights." }
                        }
                    }
                    div { class: "overlay-field",
                        label { "Accent source" }
                        Select {
                            aria_label: "Application accent source".to_string(),
                            value: snapshot.config.appearance.accent_style.clone(),
                            options: accent_options,
                            onchange: move |value| {
                                super::super::update_settings(settings, move |config| {
                                    config.appearance.accent_style = value;
                                });
                            }
                        }
                    }
                    if snapshot.config.appearance.accent_style == "Custom" {
                        ColorPicker {
                            label: "Accent color".to_string(),
                            value: snapshot.config.appearance.accent_color,
                            aria_label: "Application accent color".to_string(),
                            onchange: move |color| {
                                super::super::update_settings(settings, move |config| {
                                    config.appearance.accent_color = color;
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn UpdateCard(settings: Signal<super::super::SettingsSnapshot>, enabled: bool) -> Element {
    let mut refresh_sequence = use_signal(|| 0_u64);
    let _ = refresh_sequence();
    let status = crate::update_status_snapshot();
    let available_version = status.available_version.clone();

    rsx! {
        section { class: "sound-card update-card",
            div { class: "sound-card-title startup-row",
                div { class: "startup-copy",
                    h2 { "Check for updates" }
                    p { "Ask GitHub for the latest public release at most once per day." }
                }
                Toggle {
                    checked: enabled,
                    onchange: move |checked| {
                        super::super::update_settings(settings, |config| {
                            config.updates.check_automatically = checked;
                        });
                    }
                }
            }

            dl { class: "update-status-grid",
                div { class: "update-status-row",
                    dt { "Latest release" }
                    dd { "{status.latest_release}" }
                }
                div { class: "update-status-row",
                    dt { "Last successful check" }
                    dd { "{status.last_successful_check}" }
                }
                if status.last_error != "None" {
                    div { class: "update-status-row update-error-row",
                        dt { "Last error" }
                        dd { "{status.last_error}" }
                    }
                }
            }

            div { class: "update-actions",
                button {
                    r#type: "button",
                    class: "secondary",
                    disabled: status.checking,
                    onclick: move |_| {
                        if crate::start_manual_update_check() {
                            refresh_sequence += 1;
                            spawn(async move {
                                while crate::update_check_in_progress() {
                                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                    refresh_sequence += 1;
                                }
                                refresh_sequence += 1;
                            });
                        } else {
                            super::super::set_settings_notice(
                                settings,
                                "An update check is already running.".to_string(),
                            );
                        }
                    },
                    span { class: "solar-icon button-icon icon-diagnostics" }
                    if status.checking { "Checking..." } else { "Check now" }
                }
                if available_version.is_some() {
                    button {
                        r#type: "button",
                        class: "secondary update-download",
                        onclick: move |_| {
                            if let Err(error) = crate::open_available_update() {
                                super::super::set_settings_error(
                                    settings,
                                    format!("Could not open the update: {error:#}"),
                                );
                            }
                        },
                        "Download update"
                    }
                }
            }
            p { class: "update-privacy-note",
                "No GitHub account or token is used. Downloading and installation require explicit user actions."
            }
        }
    }
}

#[component]
fn DeviceNotificationCard(
    settings: Signal<super::super::SettingsSnapshot>,
    enabled: bool,
) -> Element {
    rsx! {
        section { class: "sound-card startup-card",
            div { class: "sound-card-title startup-row",
                div { class: "startup-copy",
                    h2 { "Microphone change notifications" }
                    p { "Notify you when Windows changes or disconnects the default communications microphone." }
                }
                Toggle {
                    checked: enabled,
                    onchange: move |checked| {
                        super::super::update_settings(settings, |config| {
                            config.device_notifications.notify_changes = checked;
                        });
                    }
                }
            }
        }
    }
}

#[component]
pub(crate) fn Toggle(checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        label { class: "switch",
            input {
                r#type: "checkbox",
                checked,
                onchange: move |event| onchange.call(event.checked()),
            }
            span { class: "slider" }
        }
    }
}
