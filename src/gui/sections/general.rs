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
                p { "Startup behavior for the lightweight background process." }
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
