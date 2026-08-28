use dioxus::prelude::*;

use crate::gui::controls::{ColorPicker, Select, SelectOption};

const APP_IMAGE: Asset = asset!("/assets/muteguard.png");
type Settings = Signal<super::super::SettingsSnapshot>;

pub(crate) fn render(settings: Settings) -> Element {
    let snapshot = settings();
    let tray_icon = snapshot.config.tray_icon.clone();
    let (preview_tone_class, preview_tone_style) = super::icon_family::preview_tone(
        &tray_icon.status_style,
        &tray_icon.status_color,
        snapshot.muted,
    );

    rsx! {
        section { class: "overlay-panel", id: "tray-icon-overview",
            div { class: "overlay-header section-head-row",
                div {
                    h1 { "Tray" }
                    p { "Keep the current Windows microphone state visible at a glance." }
                }
            }

            div { class: "settings-card-grid",
                TrayStyleCard {
                    settings,
                    tray_icon,
                    preview_tone_class,
                    preview_tone_style,
                }

                section { class: "sound-card",
                    h2 { "Tray menu" }
                    p { "Left-click the tray icon to open Settings. Right-click to mute or unmute the microphone, open Settings or exit MuteGuard." }
                }
            }
        }
    }
}

#[component]
fn TrayStyleCard(
    settings: Settings,
    tray_icon: crate::TrayIconConfig,
    preview_tone_class: &'static str,
    preview_tone_style: String,
) -> Element {
    let status_controls_class = if tray_icon.variant == "StatusMic" {
        "overlay-collapse open"
    } else {
        "overlay-collapse"
    };

    rsx! {
        section { class: "sound-card overlay-appearance",
            div { class: "overlay-field",
                label { "Tray icon style" }
                TrayVariantGrid {
                    settings,
                    tray_icon: tray_icon.clone(),
                    preview_tone_class,
                    preview_tone_style: preview_tone_style.clone(),
                }
            }

            div { class: status_controls_class,
                div { class: "overlay-collapse-inner",
                    TrayMicrophoneControls {
                        settings,
                        tray_icon,
                        preview_tone_class,
                        preview_tone_style,
                    }
                }
            }
        }
    }
}

#[component]
fn TrayVariantGrid(
    settings: Settings,
    tray_icon: crate::TrayIconConfig,
    preview_tone_class: &'static str,
    preview_tone_style: String,
) -> Element {
    rsx! {
        div { class: "overlay-variant-grid tray-icon-variant-grid",
            button {
                class: variant_button_class(tray_icon.variant == "Logo"),
                aria_pressed: tray_icon.variant == "Logo",
                onclick: move |_| set_tray_variant(settings, "Logo"),
                span { class: "overlay-icon-preview overlay-variant-preview tray-logo-preview",
                    img { src: APP_IMAGE, alt: "MuteGuard" }
                }
                span { "Logo" }
            }
            button {
                class: variant_button_class(tray_icon.variant == "StatusMic"),
                aria_pressed: tray_icon.variant == "StatusMic",
                onclick: move |_| set_tray_variant(settings, "StatusMic"),
                span {
                    class: "overlay-icon-preview overlay-variant-preview {preview_tone_class}",
                    style: "{preview_tone_style}",
                    span {
                        class: "solar-icon",
                        style: format!(
                            "--icon: url('{}');",
                            crate::overlay_icons::overlay_icon_css_url("fluent", false),
                        )
                    }
                }
                span { "Mic status" }
            }
            button {
                class: variant_button_class(tray_icon.variant == "ColorDot"),
                aria_pressed: tray_icon.variant == "ColorDot",
                onclick: move |_| set_tray_variant(settings, "ColorDot"),
                span { class: "overlay-icon-preview overlay-variant-preview dot", span {} }
                span { "Color dot" }
            }
        }
    }
}

#[component]
fn TrayMicrophoneControls(
    settings: Settings,
    tray_icon: crate::TrayIconConfig,
    preview_tone_class: &'static str,
    preview_tone_style: String,
) -> Element {
    rsx! {
        super::icon_family::IconFamilyPicker {
            value: tray_icon.icon_pair.clone(),
            preview_muted: false,
            preview_tone_class,
            preview_tone_style,
            onchange: move |value| {
                super::super::update_settings(settings, move |config| {
                    config.tray_icon.icon_pair = value;
                });
            },
        }

        div { class: "overlay-field",
            label { "Icon color" }
            Select {
                aria_label: "Tray status color".to_string(),
                value: tray_icon.status_style.clone(),
                options: status_style_options(),
                onchange: move |value| {
                    super::super::update_settings(settings, move |config| {
                        config.tray_icon.status_style = value;
                    });
                }
            }
        }
        if tray_icon.status_style == "Custom" {
            ColorPicker {
                label: "Color".to_string(),
                value: tray_icon.status_color,
                aria_label: "Tray microphone icon color".to_string(),
                onchange: move |color| {
                    super::super::update_settings(settings, move |config| {
                        config.tray_icon.status_color = color;
                    });
                }
            }
        }
    }
}

fn variant_button_class(active: bool) -> &'static str {
    if active {
        "overlay-icon-option overlay-variant-option active"
    } else {
        "overlay-icon-option overlay-variant-option"
    }
}

fn set_tray_variant(settings: Settings, variant: &'static str) {
    super::super::update_settings(settings, move |config| {
        config.tray_icon.variant = variant.to_string();
    });
}

fn status_style_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("Custom", "Colored").icon("icon-palette"),
        SelectOption::new("Monochrome", "Monochrome").icon("icon-contrast"),
        SelectOption::new("SystemColor", crate::gui::SYSTEM_COLOR_LABEL).icon("icon-widget"),
    ]
}
