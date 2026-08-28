use dioxus::prelude::*;

#[component]
pub(super) fn IconFamilyPicker(
    value: String,
    preview_muted: bool,
    preview_tone_class: &'static str,
    preview_tone_style: String,
    onchange: EventHandler<String>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let selected_extra = crate::overlay_icons::extra_overlay_icon_pair(&value);

    rsx! {
        div { class: "overlay-field overlay-icon-field",
            label { "Icon family" }
            div { class: "overlay-icon-grid overlay-icon-grid-primary",
                for pair in crate::overlay_icons::featured_overlay_icon_pairs() {
                    IconFamilyOption {
                        pair: *pair,
                        selected: value == pair.id,
                        preview_muted,
                        preview_tone_class,
                        preview_tone_style: preview_tone_style.clone(),
                        onchange,
                    }
                }
                button {
                    r#type: "button",
                    class: if expanded() { "overlay-icon-option overlay-icon-toggle expanded" } else { "overlay-icon-option overlay-icon-toggle" },
                    aria_expanded: expanded(),
                    title: if expanded() { "Show fewer styles" } else { "Show more styles" },
                    onclick: move |_| expanded.set(!expanded()),
                    span { class: "overlay-icon-preview",
                        span { class: "solar-icon icon-chevron-down overlay-icon-toggle-glyph" }
                    }
                    span { if expanded() { "Less" } else { "More" } }
                }
            }
            if expanded() {
                div { class: "overlay-icon-grid overlay-icon-grid-expanded",
                    for pair in crate::overlay_icons::extra_overlay_icon_pairs() {
                        IconFamilyOption {
                            pair: *pair,
                            selected: value == pair.id,
                            preview_muted,
                            preview_tone_class,
                            preview_tone_style: preview_tone_style.clone(),
                            onchange,
                        }
                    }
                }
            } else if let Some(pair) = selected_extra {
                span { class: "overlay-icon-selected-label", "Selected style" }
                div { class: "overlay-icon-grid overlay-icon-grid-selected",
                    IconFamilyOption {
                        pair,
                        selected: true,
                        preview_muted,
                        preview_tone_class,
                        preview_tone_style,
                        onchange,
                    }
                }
            }
        }
    }
}

#[component]
fn IconFamilyOption(
    pair: crate::overlay_icons::OverlayIconPair,
    selected: bool,
    preview_muted: bool,
    preview_tone_class: &'static str,
    preview_tone_style: String,
    onchange: EventHandler<String>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if selected { "overlay-icon-option active" } else { "overlay-icon-option" },
            aria_pressed: selected,
            onclick: move |_| onchange.call(pair.id.to_string()),
            title: pair.label,
            span {
                class: "overlay-icon-preview {preview_tone_class}",
                style: "{preview_tone_style}",
                span {
                    class: "solar-icon",
                    style: format!(
                        "--icon: url('{}');",
                        crate::overlay_icons::overlay_icon_css_url(pair.id, preview_muted),
                    )
                }
            }
            span { "{pair.label}" }
        }
    }
}

pub(super) fn preview_tone(style: &str, color: &str, muted: bool) -> (&'static str, String) {
    match style {
        "Custom" => ("custom", format!("--preview-color: {color};")),
        "Monochrome" => ("monochrome", String::new()),
        "SystemColor" => ("system", String::new()),
        _ if muted => ("muted", String::new()),
        _ => ("live", String::new()),
    }
}
