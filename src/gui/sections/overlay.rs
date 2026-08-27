use std::time::Duration;

use dioxus::prelude::*;

use crate::gui::controls::{ColorPicker, MultiSelect, Range, Select, SelectOption};

type Settings = Signal<super::super::SettingsSnapshot>;

pub(crate) fn render(mut settings: Settings) -> Element {
    let snapshot = settings();

    use_effect(move || {
        let mut current = settings();
        if current.overlay_displays.is_some() && current.system_fonts.is_some() {
            return;
        }
        if current.overlay_displays.is_none() {
            current.overlay_displays = Some(crate::overlay_displays());
        }
        if current.system_fonts.is_none() {
            current.system_fonts = Some(crate::system_fonts());
        }
        settings.set(current);
    });

    let overlay = snapshot.config.overlay.clone();
    let (selected_displays, display_options) = display_selection(&snapshot, &overlay);
    let font_options = font_options(&snapshot, &overlay.text_font);

    rsx! {
        section { class: "overlay-panel", id: "overlay-overview",
            div { class: "overlay-header section-head-row",
                div {
                    h1 { "Overlay" }
                    p { "A click-through status indicator driven directly by Core Audio events." }
                }
                super::general::Toggle {
                    checked: overlay.enabled,
                    onchange: move |checked| {
                        super::super::update_settings(settings, |config| {
                            config.overlay.enabled = checked;
                        });
                    }
                }
            }

            div { class: "settings-card-grid overlay-card-grid",
                OverlayBehaviorCard {
                    settings,
                    overlay: overlay.clone(),
                    selected_displays,
                    display_options,
                }
                OverlayContentCard {
                    settings,
                    overlay: overlay.clone(),
                    font_options,
                }
                OverlayBackgroundCard { settings, overlay }
            }
        }
    }
}

#[component]
fn OverlayBehaviorCard(
    settings: Settings,
    overlay: crate::OverlayConfig,
    selected_displays: Vec<String>,
    display_options: Vec<SelectOption>,
) -> Element {
    rsx! {
        section { class: "sound-card",
            div { class: "overlay-field",
                label { "Visibility" }
                Select {
                    aria_label: "Overlay visibility".to_string(),
                    value: overlay.visibility.clone(),
                    options: visibility_options(),
                    onchange: move |value| {
                        super::super::update_settings(settings, move |config| {
                            config.overlay.visibility = value;
                        });
                    }
                }
            }

            if overlay.visibility == "AfterToggle" {
                Range {
                    label: "Temporary duration".to_string(),
                    value: overlay.duration_secs.to_string(),
                    min: "0.5".to_string(),
                    max: "10".to_string(),
                    step: "0.5".to_string(),
                    value_suffix: "s".to_string(),
                    value_decimals: 1,
                    onchange: move |value: String| {
                        if let Ok(value) = value.parse::<f64>() {
                            super::super::update_settings(settings, |config| {
                                config.overlay.duration_secs = value.clamp(0.5, 10.0);
                            });
                        }
                    }
                }
            }

            div { class: "overlay-field",
                label { "Monitor" }
                MultiSelect {
                    aria_label: "Overlay monitors".to_string(),
                    values: selected_displays,
                    options: display_options,
                    onchange: move |values: Vec<String>| {
                        let Some(primary_display) = values.first().cloned() else {
                            return;
                        };
                        super::super::update_settings(settings, move |config| {
                            config.overlay.display = primary_display;
                            config.overlay.displays = values;
                        });
                    }
                }
            }

            OverlayPositionPicker {
                settings,
                overlay,
            }
        }
    }
}

#[component]
fn OverlayPositionPicker(settings: Settings, overlay: crate::OverlayConfig) -> Element {
    let mut preview_enabled = use_signal(|| false);
    let selected_x = position_anchor(overlay.position_x);
    let selected_y = position_anchor(overlay.position_y);
    let preview_icon_url = crate::overlay_icons::overlay_icon_css_url(
        &overlay.icon_pair,
        overlay.visibility != "WhenUnmuted",
    );

    use_drop(move || crate::request_overlay_preview(false));
    use_effect(move || {
        let enabled = preview_enabled();
        crate::request_overlay_preview(enabled);
        if enabled {
            spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if !preview_enabled() {
                        break;
                    }
                    crate::request_overlay_preview(true);
                }
            });
        }
    });

    rsx! {
        div { class: "overlay-field",
            div { class: "overlay-field-heading",
                span { "Position" }
                label { class: "hotkey-ignore-modifiers",
                    input {
                        r#type: "checkbox",
                        checked: preview_enabled(),
                        onchange: move |event| preview_enabled.set(event.checked()),
                    }
                    span { "Preview" }
                }
            }
            div {
                class: "overlay-position-picker",
                role: "group",
                aria_label: "Overlay position on monitor",
                for (x, y, label) in POSITION_ANCHORS {
                    button {
                        key: "{x}-{y}",
                        r#type: "button",
                        class: if selected_x == x && selected_y == y { "overlay-position-option active" } else { "overlay-position-option" },
                        aria_label: "{label}",
                        title: "{label}",
                        aria_pressed: selected_x == x && selected_y == y,
                        onclick: move |_| {
                            super::super::update_settings(settings, |config| {
                                config.overlay.position_x = x;
                                config.overlay.position_y = y;
                            });
                        },
                        if selected_x == x && selected_y == y {
                            span { class: "overlay-position-selected-preview",
                                span {
                                    class: "solar-icon overlay-position-preview-icon",
                                    style: format!("--icon: url('{}');", preview_icon_url),
                                }
                                span { class: "overlay-position-preview-line" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn OverlayContentCard(
    settings: Settings,
    overlay: crate::OverlayConfig,
    font_options: Vec<SelectOption>,
) -> Element {
    let shows_icon = matches!(overlay.variant.as_str(), "MicIcon" | "IconText");
    let shows_text = matches!(overlay.variant.as_str(), "Text" | "IconText");

    rsx! {
        section { class: "sound-card overlay-appearance",
            h2 { "Content" }
            div { class: "overlay-field",
                label { "Style" }
                Select {
                    aria_label: "Overlay content style".to_string(),
                    value: overlay.variant.clone(),
                    options: content_variant_options(),
                    onchange: move |value| {
                        super::super::update_settings(settings, move |config| {
                            config.overlay.variant = value;
                        });
                    }
                }
            }

            if shows_icon {
                OverlayIconControls { settings, overlay: overlay.clone() }
            }
            if shows_text {
                OverlayTextControls {
                    settings,
                    overlay: overlay.clone(),
                    font_options,
                }
            }

            Range {
                label: "Scale".to_string(),
                value: overlay.scale.to_string(),
                min: "10".to_string(),
                max: "400".to_string(),
                step: "5".to_string(),
                value_suffix: "%".to_string(),
                onchange: move |value: String| {
                    if let Ok(value) = value.parse::<u32>() {
                        super::super::update_settings(settings, |config| {
                            config.overlay.scale = value.clamp(10, 400);
                        });
                    }
                }
            }

            Range {
                label: "Content opacity".to_string(),
                value: overlay.content_opacity.to_string(),
                min: "20".to_string(),
                max: "100".to_string(),
                step: "5".to_string(),
                value_suffix: "%".to_string(),
                onchange: move |value: String| {
                    if let Ok(value) = value.parse::<u8>() {
                        super::super::update_settings(settings, |config| {
                            config.overlay.content_opacity = value.clamp(20, 100);
                        });
                    }
                }
            }
        }
    }
}

#[component]
fn OverlayIconControls(settings: Settings, overlay: crate::OverlayConfig) -> Element {
    let preview_muted = overlay.visibility == "WhenMuted";
    let icon_options = crate::overlay_icons::featured_overlay_icon_pairs()
        .iter()
        .chain(crate::overlay_icons::extra_overlay_icon_pairs().iter())
        .map(|pair| {
            SelectOption::new(pair.id, pair.label).icon_url(
                crate::overlay_icons::overlay_icon_css_url(pair.id, preview_muted),
            )
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "overlay-field",
            label { "Icon" }
            Select {
                aria_label: "Overlay icon".to_string(),
                value: overlay.icon_pair.clone(),
                options: icon_options,
                onchange: move |value| {
                    super::super::update_settings(settings, move |config| {
                        config.overlay.icon_pair = value;
                    });
                }
            }
        }
        div { class: "overlay-field",
            label { "Icon color" }
            Select {
                aria_label: "Overlay icon color".to_string(),
                value: overlay.icon_style.clone(),
                options: icon_style_options(),
                onchange: move |value| {
                    super::super::update_settings(settings, move |config| {
                        config.overlay.icon_style = value;
                    });
                }
            }
        }
        if overlay.icon_style == "Custom" {
            ColorPicker {
                label: "Color".to_string(),
                value: overlay.icon_color,
                aria_label: "Overlay icon color".to_string(),
                onchange: move |value| {
                    super::super::update_settings(settings, move |config| {
                        config.overlay.icon_color = value;
                    });
                }
            }
        }
    }
}

#[component]
fn OverlayTextControls(
    settings: Settings,
    overlay: crate::OverlayConfig,
    font_options: Vec<SelectOption>,
) -> Element {
    rsx! {
        label { class: "overlay-field",
            span { "Muted label" }
            input {
                value: overlay.muted_label.clone(),
                onchange: move |event| {
                    let value = event.value();
                    super::super::update_settings(settings, move |config| {
                        config.overlay.muted_label = value;
                    });
                }
            }
        }
        label { class: "overlay-field",
            span { "Unmuted label" }
            input {
                value: overlay.unmuted_label,
                onchange: move |event| {
                    let value = event.value();
                    super::super::update_settings(settings, move |config| {
                        config.overlay.unmuted_label = value;
                    });
                }
            }
        }
        div { class: "overlay-field",
            label { "Font" }
            Select {
                aria_label: "Overlay text font".to_string(),
                value: overlay.text_font.clone(),
                options: font_options,
                searchable: true,
                onchange: move |value| {
                    super::super::update_settings(settings, move |config| {
                        config.overlay.text_font = value;
                    });
                }
            }
        }
        Range {
            label: "Font weight".to_string(),
            value: overlay.text_font_weight.to_string(),
            min: "100".to_string(),
            max: "900".to_string(),
            step: "100".to_string(),
            onchange: move |value: String| {
                if let Ok(value) = value.parse::<u16>() {
                    super::super::update_settings(settings, |config| {
                        config.overlay.text_font_weight = value.clamp(100, 900);
                    });
                }
            }
        }
    }
}

#[component]
fn OverlayBackgroundCard(settings: Settings, overlay: crate::OverlayConfig) -> Element {
    rsx! {
        section { class: "sound-card overlay-appearance",
            h2 { "Background" }
            div { class: "overlay-field",
                label { "Background style" }
                Select {
                    aria_label: "Overlay background style".to_string(),
                    value: overlay.background_style.clone(),
                    options: background_options(),
                    onchange: move |value| {
                        super::super::update_settings(settings, move |config| {
                            config.overlay.background_style = value;
                        });
                    }
                }
            }
            Range {
                label: "Background opacity".to_string(),
                value: overlay.background_opacity.to_string(),
                min: "0".to_string(),
                max: "100".to_string(),
                step: "5".to_string(),
                value_suffix: "%".to_string(),
                onchange: move |value: String| {
                    if let Ok(value) = value.parse::<u8>() {
                        super::super::update_settings(settings, |config| {
                            config.overlay.background_opacity = value.min(100);
                        });
                    }
                }
            }
            Range {
                label: "Corner radius".to_string(),
                value: overlay.border_radius.to_string(),
                min: "0".to_string(),
                max: "24".to_string(),
                step: "1".to_string(),
                value_suffix: "px".to_string(),
                onchange: move |value: String| {
                    if let Ok(value) = value.parse::<u8>() {
                        super::super::update_settings(settings, |config| {
                            config.overlay.border_radius = value.min(24);
                        });
                    }
                }
            }
            label { class: "hotkey-ignore-modifiers",
                input {
                    r#type: "checkbox",
                    checked: overlay.show_border,
                    onchange: move |event| {
                        let checked = event.checked();
                        super::super::update_settings(settings, |config| {
                            config.overlay.show_border = checked;
                        });
                    }
                }
                span { "Show border" }
            }
            if overlay.show_border {
                ColorPicker {
                    label: "Border color".to_string(),
                    value: overlay.border_color,
                    aria_label: "Overlay border color".to_string(),
                    onchange: move |value| {
                        super::super::update_settings(settings, move |config| {
                            config.overlay.border_color = value;
                        });
                    }
                }
            }
        }
    }
}

fn display_selection(
    snapshot: &super::super::SettingsSnapshot,
    overlay: &crate::OverlayConfig,
) -> (Vec<String>, Vec<SelectOption>) {
    let mut options = snapshot
        .overlay_displays
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|display| {
            SelectOption::new(display.id.clone(), display.label.clone())
                .detail(display.detail.clone())
                .icon("icon-monitor")
        })
        .collect::<Vec<_>>();
    let selected = if overlay.displays.is_empty() {
        vec![overlay.display.clone()]
    } else {
        overlay.displays.clone()
    };

    for display_id in selected.iter().rev() {
        if !options.iter().any(|option| option.value == *display_id) {
            options.insert(
                0,
                SelectOption::new(display_id.clone(), "Unavailable display")
                    .detail("Reconnect this monitor or remove it from the selection")
                    .icon("icon-monitor"),
            );
        }
    }

    (selected, options)
}

fn font_options(
    snapshot: &super::super::SettingsSnapshot,
    selected_font: &str,
) -> Vec<SelectOption> {
    let mut options = snapshot
        .system_fonts
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|font| {
            SelectOption::new(font.family.clone(), font.family.clone())
                .font_family(font.family.clone())
        })
        .collect::<Vec<_>>();
    if !options
        .iter()
        .any(|option| option.value.eq_ignore_ascii_case(selected_font))
    {
        options.insert(
            0,
            SelectOption::new(selected_font, selected_font).font_family(selected_font),
        );
    }
    options
}

fn visibility_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("Always", "Always visible").icon("icon-widget"),
        SelectOption::new("WhenMuted", "Visible when muted")
            .icon_url(crate::overlay_icons::overlay_icon_css_url("fluent", true)),
        SelectOption::new("WhenUnmuted", "Visible when unmuted")
            .icon_url(crate::overlay_icons::overlay_icon_css_url("fluent", false)),
        SelectOption::new("AfterToggle", "Temporarily after a change").icon("icon-clock-circle"),
    ]
}

fn content_variant_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("MicIcon", "Microphone icon")
            .icon_url(crate::overlay_icons::overlay_icon_css_url("fluent", false)),
        SelectOption::new("IconText", "Icon and text").icon("icon-widget"),
        SelectOption::new("Text", "Text only").icon("icon-record"),
        SelectOption::new("Dot", "Status dot").icon("icon-record"),
    ]
}

fn icon_style_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("Custom", "Colored").icon("icon-palette"),
        SelectOption::new("Monochrome", "Monochrome").icon("icon-contrast"),
        SelectOption::new("SystemColor", "System color").icon("icon-widget"),
    ]
}

fn background_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("Dark", "Dark").icon("icon-moon"),
        SelectOption::new("Light", "Light").icon("icon-sun"),
        SelectOption::new("Transparent", "Transparent").icon("icon-contrast"),
    ]
}

const POSITION_ANCHORS: [(f64, f64, &str); 9] = [
    (0.0, 0.0, "Top left"),
    (50.0, 0.0, "Top center"),
    (100.0, 0.0, "Top right"),
    (0.0, 50.0, "Middle left"),
    (50.0, 50.0, "Center"),
    (100.0, 50.0, "Middle right"),
    (0.0, 100.0, "Bottom left"),
    (50.0, 100.0, "Bottom center"),
    (100.0, 100.0, "Bottom right"),
];

fn position_anchor(value: f64) -> f64 {
    if value < 25.0 {
        0.0
    } else if value > 75.0 {
        100.0
    } else {
        50.0
    }
}
