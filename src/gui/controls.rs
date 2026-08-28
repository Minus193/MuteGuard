use dioxus::prelude::*;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

static NEXT_SELECT_ID: AtomicUsize = AtomicUsize::new(1);
const COLOR_PRESETS: [&str; 8] = [
    "#FFFFFF", "#BDC3C8", "#222F3D", "#7E40FD", "#2980B9", "#F39C19", "#2ECC70", "#E84B3C",
];

#[component]
pub fn ColorPicker(
    label: String,
    value: String,
    onchange: EventHandler<String>,
    aria_label: String,
) -> Element {
    let initial_value = canonical_hex_color(&value).unwrap_or_else(|| "#7C83FF".to_string());
    let initial_hsv = hex_to_hsv(&initial_value).unwrap_or((250.0, 48.0, 100.0));
    let mut draft = use_signal(|| initial_value.clone());
    let last_committed = use_signal(|| initial_value.clone());
    let mut picker_open = use_signal(|| false);
    let mut hue = use_signal(|| initial_hsv.0);
    let mut saturation = use_signal(|| initial_hsv.1);
    let mut brightness = use_signal(|| initial_hsv.2);
    let mut spectrum_size = use_signal(|| (0.0_f64, 0.0_f64));
    let mut spectrum_dragging = use_signal(|| false);
    let current_draft = draft();
    let valid_draft = canonical_hex_color(&current_draft);
    let preview_color = valid_draft
        .clone()
        .or_else(|| canonical_hex_color(&value))
        .unwrap_or_else(|| "#7C83FF".to_string());
    let text_class = if valid_draft.is_some() {
        "ui-color-text"
    } else {
        "ui-color-text invalid"
    };
    let blur_value = value.clone();
    let escape_value = value;
    let selected_color = preview_color.clone();
    let custom_selected = !COLOR_PRESETS.contains(&selected_color.as_str());
    let (red, green, blue) = hex_to_rgb(&preview_color).unwrap_or((124, 131, 255));
    let swatch_icon_class = if color_prefers_dark_foreground(red, green, blue) {
        "ui-color-swatch-icon dark"
    } else {
        "ui-color-swatch-icon light"
    };
    let picker_class = if picker_open() {
        "ui-color-picker open"
    } else {
        "ui-color-picker"
    };
    let hue_value = hue();
    let saturation_value = saturation();
    let brightness_value = brightness();
    let custom_active = custom_selected || picker_open();
    let spectrum_marker = format!(
        "--spectrum-x: {:.2}%; --spectrum-y: {:.2}%;",
        saturation_value,
        100.0 - brightness_value
    );

    rsx! {
        div { class: "ui-color-field",
            span { class: "ui-color-label", "{label}" }
            div {
                class: "{picker_class}",
                style: "--picker-color: {preview_color}; --picker-hue: {hue_value}deg; --picker-saturation: {saturation_value}%; --picker-brightness: {brightness_value}%;",
                button {
                    r#type: "button",
                    class: "ui-color-swatch",
                    title: "Open color studio",
                    aria_label: "Open {aria_label}",
                    aria_expanded: picker_open(),
                    onclick: move |_| picker_open.set(!picker_open()),
                    span { class: "ui-color-swatch-color", aria_hidden: "true" }
                    span { class: "solar-icon icon-palette {swatch_icon_class}", aria_hidden: "true" }
                }
                div { class: "ui-color-entry",
                    span { class: "ui-color-entry-caption", "HEX" }
                    input {
                        class: "{text_class}",
                        r#type: "text",
                        value: "{current_draft}",
                        maxlength: "7",
                        autocomplete: "off",
                        spellcheck: "false",
                        "aria-label": "{aria_label}",
                        "aria-invalid": if valid_draft.is_some() { "false" } else { "true" },
                        oninput: move |event| {
                            let next = event.value();
                            draft.set(next);
                        },
                        onblur: move |_| {
                            if let Some(color) = canonical_hex_color(&draft()) {
                                sync_hsv_signals(&color, hue, saturation, brightness);
                                draft.set(color.clone());
                                commit_picker_color(color, last_committed, onchange);
                            } else {
                                draft.set(
                                    canonical_hex_color(&blur_value)
                                        .unwrap_or_else(|| "#7C83FF".to_string()),
                                );
                            }
                        },
                        onkeydown: move |event| {
                            match event.data().key().to_string().as_str() {
                                "Enter" => {
                                    event.prevent_default();
                                    if let Some(color) = canonical_hex_color(&draft()) {
                                        sync_hsv_signals(&color, hue, saturation, brightness);
                                        draft.set(color.clone());
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                }
                                "Escape" => {
                                    event.prevent_default();
                                    let color = canonical_hex_color(&escape_value)
                                        .unwrap_or_else(|| "#7C83FF".to_string());
                                    sync_hsv_signals(&color, hue, saturation, brightness);
                                    draft.set(color);
                                }
                                _ => {}
                            }
                        }
                    }
                    span {
                        class: "ui-color-entry-status",
                        aria_live: "polite",
                        if valid_draft.is_some() { "Exact color" } else { "Use #RRGGBB" }
                    }
                }
                if picker_open() {
                    div { class: "ui-color-studio",
                        div { class: "ui-color-studio-head",
                            div {
                                span { class: "ui-color-studio-kicker", "COLOR STUDIO" }
                                strong { "Tune the exact shade" }
                            }
                            button {
                                r#type: "button",
                                class: "ui-color-studio-done",
                                onclick: move |_| {
                                    if let Some(color) = canonical_hex_color(&draft()) {
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                    picker_open.set(false);
                                },
                                "Done"
                            }
                        }
                        div {
                            class: "ui-color-spectrum",
                            style: "{spectrum_marker}",
                            role: "group",
                            tabindex: "0",
                            aria_label: "Saturation and brightness",
                            onmounted: move |event| async move {
                                if let Ok(rect) = event.data().get_client_rect().await {
                                    spectrum_size.set((rect.size.width, rect.size.height));
                                }
                            },
                            onpointerdown: move |event| {
                                event.prevent_default();
                                spectrum_dragging.set(true);
                                let point = event.data().element_coordinates();
                                preview_spectrum_color(
                                    point.x,
                                    point.y,
                                    spectrum_size(),
                                    hue(),
                                    saturation,
                                    brightness,
                                    draft,
                                );
                            },
                            onpointermove: move |event| {
                                if spectrum_dragging() {
                                    event.prevent_default();
                                    let point = event.data().element_coordinates();
                                    preview_spectrum_color(
                                        point.x,
                                        point.y,
                                        spectrum_size(),
                                        hue(),
                                        saturation,
                                        brightness,
                                        draft,
                                    );
                                }
                            },
                            onpointerup: move |event| {
                                if spectrum_dragging() {
                                    let point = event.data().element_coordinates();
                                    if let Some(color) = preview_spectrum_color(
                                        point.x,
                                        point.y,
                                        spectrum_size(),
                                        hue(),
                                        saturation,
                                        brightness,
                                        draft,
                                    ) {
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                    spectrum_dragging.set(false);
                                }
                            },
                            onpointerleave: move |_| {
                                if spectrum_dragging() {
                                    spectrum_dragging.set(false);
                                    if let Some(color) = canonical_hex_color(&draft()) {
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                }
                            },
                            onpointercancel: move |_| {
                                spectrum_dragging.set(false);
                                if let Some(color) = canonical_hex_color(&draft()) {
                                    commit_picker_color(color, last_committed, onchange);
                                }
                            },
                            span { class: "ui-color-spectrum-marker", aria_hidden: "true" }
                        }
                        div { class: "ui-color-channel-grid", aria_label: "RGB preview",
                            span { strong { "{red}" } small { "R" } }
                            span { strong { "{green}" } small { "G" } }
                            span { strong { "{blue}" } small { "B" } }
                        }
                        label { class: "ui-color-tune hue",
                            span { "Hue" }
                            output { "{hue_value:.0}°" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "360",
                                step: "1",
                                value: "{hue_value}",
                                oninput: move |event| {
                                    if let Ok(next) = event.value().parse::<f64>() {
                                        hue.set(next);
                                        draft.set(hsv_to_hex(next, saturation(), brightness()));
                                    }
                                },
                                onchange: move |event| {
                                    if let Ok(next) = event.value().parse::<f64>() {
                                        hue.set(next);
                                        let color = hsv_to_hex(next, saturation(), brightness());
                                        draft.set(color.clone());
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                }
                            }
                        }
                        label { class: "ui-color-tune saturation",
                            span { "Saturation" }
                            output { "{saturation_value:.0}%" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "100",
                                step: "1",
                                value: "{saturation_value}",
                                oninput: move |event| {
                                    if let Ok(next) = event.value().parse::<f64>() {
                                        saturation.set(next);
                                        draft.set(hsv_to_hex(hue(), next, brightness()));
                                    }
                                },
                                onchange: move |event| {
                                    if let Ok(next) = event.value().parse::<f64>() {
                                        saturation.set(next);
                                        let color = hsv_to_hex(hue(), next, brightness());
                                        draft.set(color.clone());
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                }
                            }
                        }
                        label { class: "ui-color-tune brightness",
                            span { "Brightness" }
                            output { "{brightness_value:.0}%" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "100",
                                step: "1",
                                value: "{brightness_value}",
                                oninput: move |event| {
                                    if let Ok(next) = event.value().parse::<f64>() {
                                        brightness.set(next);
                                        draft.set(hsv_to_hex(hue(), saturation(), next));
                                    }
                                },
                                onchange: move |event| {
                                    if let Ok(next) = event.value().parse::<f64>() {
                                        brightness.set(next);
                                        let color = hsv_to_hex(hue(), saturation(), next);
                                        draft.set(color.clone());
                                        commit_picker_color(color, last_committed, onchange);
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "ui-color-presets", aria_label: "Color presets",
                    button {
                        r#type: "button",
                        class: if custom_active { "ui-color-preset custom selected" } else { "ui-color-preset custom" },
                        style: "--preset-color: {preview_color};",
                        title: "Custom color",
                        aria_label: "Edit custom color",
                        aria_pressed: custom_active,
                        onclick: move |_| picker_open.set(true),
                        span { class: "solar-icon icon-palette", aria_hidden: "true" }
                    }
                    for preset in COLOR_PRESETS {
                        {
                            let color = preset.to_string();
                            let is_selected = !picker_open() && color == selected_color;
                            let next_color = color.clone();
                            rsx! {
                                button {
                                    key: "color-preset-{color}",
                                    r#type: "button",
                                    class: if is_selected { "ui-color-preset selected" } else { "ui-color-preset" },
                                    style: "--preset-color: {color};",
                                    title: "{color}",
                                    aria_label: "Use preset color {color}",
                                    aria_pressed: is_selected,
                                    onclick: move |_| {
                                        picker_open.set(false);
                                        sync_hsv_signals(&next_color, hue, saturation, brightness);
                                        draft.set(next_color.clone());
                                        commit_picker_color(
                                            next_color.clone(),
                                            last_committed,
                                            onchange,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub detail: Option<String>,
    pub icon_class: Option<String>,
    pub icon_url: Option<String>,
    pub font_family: Option<String>,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            detail: None,
            icon_class: None,
            icon_url: None,
            font_family: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn icon(mut self, icon_class: impl Into<String>) -> Self {
        self.icon_class = Some(icon_class.into());
        self
    }

    pub fn icon_url(mut self, icon_url: impl Into<String>) -> Self {
        self.icon_url = Some(icon_url.into());
        self
    }

    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        self.font_family = Some(font_family.into());
        self
    }
}

#[component]
pub fn Range(
    label: String,
    value: String,
    min: String,
    max: String,
    step: String,
    onchange: EventHandler<String>,
    #[props(default)] class: String,
    #[props(default)] label_icon: Option<String>,
    #[props(default)] value_suffix: String,
    #[props(default)] value_decimals: u8,
) -> Element {
    let mut live_value = use_signal(|| value.clone());
    let current_value = live_value();
    let numeric_value = current_value.parse::<f64>().unwrap_or_default();
    let minimum = min.parse::<f64>().unwrap_or_default();
    let maximum = max.parse::<f64>().unwrap_or(minimum + 1.0);
    let progress = if maximum > minimum {
        format!(
            "{:.4}%",
            ((numeric_value - minimum) / (maximum - minimum) * 100.0).clamp(0.0, 100.0)
        )
    } else {
        "0%".to_string()
    };
    let value_label = format!(
        "{:.*}{}",
        usize::from(value_decimals),
        numeric_value,
        value_suffix
    );

    rsx! {
        div { class: merged_class("ui-range-control", &class),
            label { class: "ui-range-shell",
                span {
                    class: "ui-range-fill",
                    style: "--range-progress: {progress};"
                }
                input {
                    class: "ui-range",
                    r#type: "range",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    value: "{current_value}",
                    oninput: move |evt| live_value.set(evt.value()),
                    onchange: move |evt| {
                        let value = evt.value();
                        live_value.set(value.clone());
                        onchange.call(value);
                    }
                }
                span {
                    class: "ui-range-dragger",
                    style: "--range-progress: {progress};"
                }
                span { class: "ui-range-copy",
                    span { class: "ui-range-label",
                        if let Some(icon) = label_icon {
                            span { class: "solar-icon ui-range-label-icon {icon}" }
                        }
                        span { "{label}" }
                    }
                    span { class: "ui-range-value", "{value_label}" }
                }
            }
        }
    }
}

#[component]
pub fn Select(
    value: String,
    options: Vec<SelectOption>,
    onchange: EventHandler<String>,
    aria_label: String,
    #[props(default = true)] show_current_detail: bool,
    #[props(default = false)] searchable: bool,
    #[props(default = false)] disabled: bool,
    #[props(default)] class: String,
) -> Element {
    let mut open = use_signal(|| false);
    let mut menu_visible = use_signal(|| false);
    let mut menu_style = use_signal(String::new);
    let mut menu_height = use_signal(|| None::<f64>);
    let mut animate_value = use_signal(|| false);
    let mut exiting_value = use_signal(|| None::<SelectOption>);
    let mut search_query = use_signal(String::new);
    let mut highlighted_index = use_signal(|| 0_usize);
    let mut open_select = use_context::<Signal<Option<String>>>();
    let select_id = use_hook(|| {
        format!(
            "ui-select-{}",
            NEXT_SELECT_ID.fetch_add(1, Ordering::Relaxed)
        )
    });
    let current = options
        .iter()
        .find(|option| option.value == value)
        .cloned()
        .or_else(|| options.first().cloned());
    let root_class = if disabled {
        merged_class("ui-select disabled", &class)
    } else if open() {
        merged_class("ui-select open", &class)
    } else {
        merged_class("ui-select", &class)
    };
    let menu_class = if menu_style().is_empty() {
        "ui-select-menu"
    } else {
        "ui-select-menu ready"
    };
    let should_render_menu = open() || menu_visible();

    let sync_select_id = select_id.clone();
    use_effect(move || {
        if open() && open_select().as_deref() != Some(sync_select_id.as_str()) {
            open.set(false);
            menu_visible.set(false);
            menu_style.set(String::new());
            menu_height.set(None);
        }
    });

    use_effect(move || {
        if open() && !disabled {
            menu_visible.set(true);
            return;
        }

        if !menu_visible() {
            menu_style.set(String::new());
            menu_height.set(None);
            return;
        }

        spawn(async move {
            tokio::time::sleep(Duration::from_millis(220)).await;
            if !open() {
                menu_visible.set(false);
                menu_style.set(String::new());
                menu_height.set(None);
            }
        });
    });

    let search_select_id = select_id.clone();
    use_effect(move || {
        let select_id = search_select_id.clone();
        if !open() || !searchable || disabled {
            return;
        }

        spawn(async move {
            let script = format!(
                r#"
const input = document.querySelector('[data-ui-select-id="{select_id}"] .ui-select-search-input');
if (input) {{
  input.focus();
  input.select();
}}
"#
            );
            let _ = dioxus::document::eval(&script).await;
        });
    });

    let filtered_options = if should_render_menu && !disabled {
        let query = search_query().trim().to_ascii_lowercase();
        options
            .iter()
            .filter(|option| {
                query.is_empty()
                    || option.label.to_ascii_lowercase().contains(&query)
                    || option.value.to_ascii_lowercase().contains(&query)
                    || option
                        .detail
                        .as_deref()
                        .is_some_and(|detail| detail.to_ascii_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let highlighted = if filtered_options.is_empty() {
        0
    } else {
        highlighted_index().min(filtered_options.len() - 1)
    };

    let mut rendered_options = Vec::new();
    if should_render_menu {
        for (option_index, option) in filtered_options.iter().cloned().enumerate() {
            let is_selected = option.value == value;
            let next_value = option.value.clone();
            let item_class = if option_index == highlighted && is_selected {
                "ui-select-item highlighted selected"
            } else if option_index == highlighted {
                "ui-select-item highlighted"
            } else if is_selected {
                "ui-select-item selected"
            } else {
                "ui-select-item"
            };
            let option_id = format!("{select_id}-option-{option_index}");
            let close_select_id = select_id.clone();
            let should_animate = next_value != value;
            let previous_value = current.clone();
            let option_label_style = select_option_label_style(option.font_family.as_deref());

            rendered_options.push(rsx! {
                div {
                    key: "select-option-{option.value}",
                    class: "{item_class}",
                    role: "presentation",
                    button {
                        id: "{option_id}",
                        r#type: "button",
                        class: "ui-select-item-button",
                        role: "option",
                        aria_selected: is_selected,
                        onclick: move |_| {
                            open.set(false);
                            search_query.set(String::new());
                            highlighted_index.set(0);
                            if open_select().as_deref() == Some(close_select_id.as_str()) {
                                open_select.set(None);
                            }
                            if should_animate {
                                exiting_value.set(previous_value.clone());
                            }
                            animate_value.set(should_animate);
                            onchange.call(next_value.clone());
                        },
                        div { class: "ui-select-item-main",
                            if let Some(icon_url) = option.icon_url.as_deref() {
                                span {
                                    class: "solar-icon ui-select-item-icon ui-select-preview-icon",
                                    style: "{select_option_icon_style(icon_url)}"
                                }
                            } else if let Some(icon_class) = option.icon_class.as_deref() {
                                span { class: "solar-icon ui-select-item-icon {icon_class}" }
                            }
                            div { class: "ui-select-item-copy",
                                span {
                                    class: "ui-select-item-label",
                                    style: "{option_label_style}",
                                    "{option.label}"
                                }
                                if let Some(detail) = option.detail.as_deref() {
                                    span { class: "ui-select-item-detail", "{detail}" }
                                }
                            }
                        }
                    }
                }
            });
        }

        if filtered_options.is_empty() {
            let empty_key = "select-empty";
            rendered_options.push(rsx! {
                div {
                    key: "{empty_key}",
                    class: "ui-select-empty",
                    "No results"
                }
            });
        }
    }

    let position_select_id = select_id.clone();
    use_effect(move || {
        let select_id = position_select_id.clone();
        if !open() || disabled {
            return;
        }
        let locked_height = menu_height();
        let locked_height_js =
            locked_height.map_or_else(|| "null".to_string(), |height| height.to_string());

        spawn(async move {
            let script = format!(
                r#"
const root = document.querySelector('[data-ui-select-id="{select_id}"]');
const trigger = root?.querySelector('.ui-select-trigger');
const list = root?.querySelector('.ui-select-list');
if (!trigger || !list) {{
  return '';
}}

const lockedHeight = {locked_height_js};

const rect = trigger.getBoundingClientRect();
const viewportWidth = window.innerWidth;
const viewportHeight = window.innerHeight;
const gutter = 12;
const gap = 8;
const width = Math.min(rect.width, viewportWidth - gutter * 2);
const left = Math.min(
  Math.max(gutter, rect.left),
  viewportWidth - gutter - width
);
const desiredHeight = Math.min(lockedHeight ?? list.scrollHeight, 320);
const spaceAbove = Math.max(0, rect.top - gutter);
const spaceBelow = Math.max(0, viewportHeight - rect.bottom - gutter);
const minComfortHeight = Math.min(desiredHeight, 140);

let placeBelow =
  spaceBelow >= desiredHeight ||
  (spaceBelow >= minComfortHeight && spaceBelow >= spaceAbove);

let height = desiredHeight;
let top = 0;
let shift = 6;
let origin = 'top center';

if (placeBelow) {{
  height = Math.min(desiredHeight, Math.max(spaceBelow, 96));
  top = rect.bottom + gap;
  shift = -6;
  if (top + height > viewportHeight - gutter) {{
    height = Math.max(96, viewportHeight - top - gutter);
  }}
  if (height < minComfortHeight && spaceAbove > spaceBelow) {{
    placeBelow = false;
  }}
}}

if (!placeBelow) {{
  height = Math.min(desiredHeight, Math.max(spaceAbove, 96));
  top = rect.top - gap - height;
  shift = 6;
  origin = 'bottom center';
  if (top < gutter) {{
    top = gutter;
    height = Math.max(96, rect.top - gap - gutter);
  }}
}}

return `left:${{left}}px;top:${{top}}px;width:${{width}}px;--ui-select-height:${{height}}px;--ui-select-shift:${{shift}}px;--ui-select-origin:${{origin}};`;
"#
            );

            if let Ok(result) = dioxus::document::eval(&script).await
                && let Some(style) = result.as_str()
            {
                if menu_height().is_none()
                    && let Some(height) = parse_select_height(style)
                {
                    menu_height.set(Some(height));
                }
                menu_style.set(style.to_string());
            }
        });
    });

    let shadow_select_id = select_id.clone();
    use_effect(move || {
        let select_id = shadow_select_id.clone();
        let _ = highlighted_index();
        let _ = search_query();
        if !open() || disabled || menu_style().is_empty() {
            return;
        }

        spawn(async move {
            let script = format!(
                r#"
const root = document.querySelector('[data-ui-select-id="{select_id}"]');
const menu = root?.querySelector('.ui-select-menu');
const list = root?.querySelector('.ui-select-list');
if (!menu || !list) {{
  return;
}}

const scrollSelectedIntoView = () => {{
  const selected = list.querySelector('.ui-select-item.selected, .ui-select-item.highlighted');
  if (!selected) {{
    return;
  }}

  const listHeight = list.clientHeight;
  const selectedTop = selected.offsetTop;
  const selectedBottom = selectedTop + selected.offsetHeight;
  const visibleTop = list.scrollTop;
  const visibleBottom = visibleTop + listHeight;

  if (selectedTop >= visibleTop && selectedBottom <= visibleBottom) {{
    return;
  }}

  const centeredTop = selectedTop - (listHeight - selected.offsetHeight) / 2;
  const maxScroll = Math.max(0, list.scrollHeight - listHeight);
  list.scrollTop = Math.min(Math.max(0, centeredTop), maxScroll);
}};

const updateShadows = () => {{
  const maxScroll = Math.max(0, list.scrollHeight - list.clientHeight);
  const canScroll = maxScroll > 1;
  const showTop = canScroll && list.scrollTop > 1;
  const showBottom = canScroll && list.scrollTop < maxScroll - 1;

  menu.setAttribute('data-scroll-top', showTop ? 'true' : 'false');
  menu.setAttribute('data-scroll-bottom', showBottom ? 'true' : 'false');
}};

if (!list.__uiSelectShadowHandler) {{
  const handler = () => updateShadows();
  list.addEventListener('scroll', handler, {{ passive: true }});
  list.__uiSelectShadowHandler = handler;
}}

if (!list.__uiSelectShadowResizeObserver) {{
  const resizeObserver = new ResizeObserver(() => updateShadows());
  resizeObserver.observe(list);
  list.__uiSelectShadowResizeObserver = resizeObserver;
}}

scrollSelectedIntoView();
updateShadows();
requestAnimationFrame(() => {{
  scrollSelectedIntoView();
  updateShadows();
}});
"#
            );

            let _ = dioxus::document::eval(&script).await;
        });
    });

    use_effect(move || {
        if !animate_value() {
            return;
        }

        spawn(async move {
            tokio::time::sleep(Duration::from_millis(310)).await;
            animate_value.set(false);
            exiting_value.set(None);
        });
    });

    let trigger_select_id = select_id.clone();
    let keyboard_trigger_select_id = select_id.clone();
    let dismiss_select_id = select_id.clone();
    let search_input_placeholder = current
        .as_ref()
        .map_or_else(|| "Search".to_string(), |option| option.label.clone());
    let filtered_option_count = filtered_options.len();
    let highlighted_option = filtered_options.get(highlighted).cloned();
    let highlighted_option_id = highlighted_option
        .as_ref()
        .map(|_| format!("{select_id}-option-{highlighted}"))
        .unwrap_or_default();
    let list_id = format!("{select_id}-list");
    let selected_option_index = options
        .iter()
        .position(|option| option.value == value)
        .unwrap_or(0);

    rsx! {
        div { class: "{root_class}", "data-ui-select-id": "{select_id}",
            if open() {
                button {
                    r#type: "button",
                    class: "ui-select-dismiss",
                    tabindex: "-1",
                    aria_hidden: "true",
                    onclick: move |_| {
                        open.set(false);
                        search_query.set(String::new());
                        highlighted_index.set(0);
                        if open_select().as_deref() == Some(dismiss_select_id.as_str()) {
                            open_select.set(None);
                        }
                    }
                }
            }

            if searchable && open() {
                div {
                    class: "ui-select-trigger ui-select-search-trigger",
                    input {
                        class: "ui-select-search-input",
                        r#type: "text",
                        role: "combobox",
                        aria_label: "{aria_label}",
                        aria_autocomplete: "list",
                        aria_controls: "{list_id}",
                        aria_expanded: "true",
                        aria_activedescendant: "{highlighted_option_id}",
                        value: "{search_query}",
                        placeholder: "{search_input_placeholder}",
                        oninput: move |evt| {
                            search_query.set(evt.value());
                            highlighted_index.set(0);
                        },
                        onkeydown: move |evt| {
                            let key = evt.data().key().to_string();
                            match key.as_str() {
                                "ArrowDown" => {
                                    evt.prevent_default();
                                    if filtered_option_count > 0 {
                                        highlighted_index.set((highlighted_index() + 1) % filtered_option_count);
                                    }
                                }
                                "ArrowUp" => {
                                    evt.prevent_default();
                                    if filtered_option_count > 0 {
                                        highlighted_index.set(
                                            if highlighted_index() == 0 {
                                                filtered_option_count - 1
                                            } else {
                                                highlighted_index() - 1
                                            }
                                        );
                                    }
                                }
                                "Enter" => {
                                    evt.prevent_default();
                                    if let Some(option) = highlighted_option.clone() {
                                        let should_animate = option.value != value;
                                        open.set(false);
                                        search_query.set(String::new());
                                        highlighted_index.set(0);
                                        open_select.set(None);
                                        if should_animate {
                                            exiting_value.set(current.clone());
                                        }
                                        animate_value.set(should_animate);
                                        onchange.call(option.value);
                                    }
                                }
                                "Escape" => {
                                    evt.prevent_default();
                                    open.set(false);
                                    search_query.set(String::new());
                                    highlighted_index.set(0);
                                    open_select.set(None);
                                }
                                _ => {}
                            }
                        }
                    }
                    span { class: "solar-icon ui-select-chevron icon-down" }
                }
            } else {
                button {
                    r#type: "button",
                    class: "ui-select-trigger",
                    disabled,
                    role: "combobox",
                    aria_label: "{aria_label}",
                    aria_haspopup: "listbox",
                    aria_controls: "{list_id}",
                    aria_expanded: if open() { "true" } else { "false" },
                    aria_activedescendant: if open() { highlighted_option_id.as_str() } else { "" },
                    onclick: move |_| {
                        if disabled {
                            return;
                        }
                        if open() {
                            open.set(false);
                            search_query.set(String::new());
                            highlighted_index.set(0);
                            open_select.set(None);
                        } else {
                            search_query.set(String::new());
                            highlighted_index.set(selected_option_index);
                            open_select.set(Some(trigger_select_id.clone()));
                            open.set(true);
                        }
                    },
                    onkeydown: move |evt| {
                        let key = evt.data().key().to_string();
                        match key.as_str() {
                            "ArrowDown" | "ArrowUp" => {
                                evt.prevent_default();
                                if !open() {
                                    search_query.set(String::new());
                                    highlighted_index.set(selected_option_index);
                                    open_select.set(Some(keyboard_trigger_select_id.clone()));
                                    open.set(true);
                                } else if filtered_option_count > 0 {
                                    let next = if key == "ArrowDown" {
                                        (highlighted_index() + 1) % filtered_option_count
                                    } else if highlighted_index() == 0 {
                                        filtered_option_count - 1
                                    } else {
                                        highlighted_index() - 1
                                    };
                                    highlighted_index.set(next);
                                }
                            }
                            "Enter" | " " if open() => {
                                evt.prevent_default();
                                if let Some(option) = highlighted_option.clone() {
                                    let should_animate = option.value != value;
                                    open.set(false);
                                    highlighted_index.set(0);
                                    open_select.set(None);
                                    if should_animate {
                                        exiting_value.set(current.clone());
                                    }
                                    animate_value.set(should_animate);
                                    onchange.call(option.value);
                                }
                            }
                            "Escape" if open() => {
                                evt.prevent_default();
                                open.set(false);
                                highlighted_index.set(0);
                                open_select.set(None);
                            }
                            _ => {}
                        }
                    },
                    div { class: "ui-select-current",
                    if current.as_ref().is_some_and(select_option_has_icon)
                        || exiting_value().as_ref().is_some_and(select_option_has_icon)
                    {
                        span { class: "ui-select-current-icon-stack",
                            if animate_value() {
                                if let Some(option) = exiting_value().as_ref() {
                                    if let Some(icon_url) = option.icon_url.as_deref() {
                                        span {
                                            key: "current-icon-url-exit-{option.value}",
                                            class: "solar-icon ui-select-current-icon ui-select-preview-icon ui-select-current-icon-exit",
                                            style: "{select_option_icon_style(icon_url)}"
                                        }
                                    } else if let Some(icon_class) = option.icon_class.as_deref() {
                                        span {
                                            key: "current-icon-exit-{option.value}",
                                            class: "solar-icon ui-select-current-icon ui-select-current-icon-exit {icon_class}"
                                        }
                                    }
                                }
                            }
                            if let Some(option) = current.as_ref() {
                                if let Some(icon_url) = option.icon_url.as_deref() {
                                    span {
                                        key: "current-icon-url-enter-{option.value}",
                                        class: if animate_value() {
                                            "solar-icon ui-select-current-icon ui-select-preview-icon ui-select-current-icon-enter"
                                        } else {
                                            "solar-icon ui-select-current-icon ui-select-preview-icon"
                                        },
                                        style: "{select_option_icon_style(icon_url)}"
                                    }
                                } else if let Some(icon_class) = option.icon_class.as_deref() {
                                    span {
                                        key: "current-icon-enter-{option.value}",
                                        class: if animate_value() {
                                            "solar-icon ui-select-current-icon ui-select-current-icon-enter {icon_class}"
                                        } else {
                                            "solar-icon ui-select-current-icon {icon_class}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "ui-select-current-copy",
                        if animate_value() {
                            if let Some(option) = exiting_value().as_ref() {
                                div {
                                    key: "current-exit-{option.value}",
                                    class: "ui-select-current-text ui-select-current-text-exit",
                                    span {
                                        class: "ui-select-current-label",
                                        style: "{select_option_label_style(option.font_family.as_deref())}",
                                        "{option.label}"
                                    }
                                    if show_current_detail {
                                        if let Some(detail) = option.detail.as_deref() {
                                            span { class: "ui-select-current-detail", "{detail}" }
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(option) = current.as_ref() {
                            div {
                                key: "current-enter-{option.value}",
                                class: if animate_value() { "ui-select-current-text ui-select-current-text-enter" } else { "ui-select-current-text" },
                                span {
                                    class: "ui-select-current-label",
                                    style: "{select_option_label_style(option.font_family.as_deref())}",
                                    "{option.label}"
                                }
                                if show_current_detail {
                                    if let Some(detail) = option.detail.as_deref() {
                                        span { class: "ui-select-current-detail", "{detail}" }
                                    }
                                }
                            }
                        }
                    }
                    }
                    span { class: "solar-icon ui-select-chevron icon-down" }
                }
            }

            if should_render_menu {
                div { class: "{menu_class}", style: "{menu_style}",
                    div { class: "ui-select-scroll-shadow top", aria_hidden: "true" }
                    div { class: "ui-select-scroll-shadow bottom", aria_hidden: "true" }
                    div { class: "ui-select-list", id: "{list_id}", role: "listbox",
                        for item in rendered_options {
                            {item}
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MultiSelect(
    values: Vec<String>,
    options: Vec<SelectOption>,
    onchange: EventHandler<Vec<String>>,
    aria_label: String,
    #[props(default = true)] required: bool,
    #[props(default)] class: String,
) -> Element {
    let mut open = use_signal(|| false);
    let mut menu_style = use_signal(String::new);
    let mut open_select = use_context::<Signal<Option<String>>>();
    let select_id = use_hook(|| {
        format!(
            "ui-select-{}",
            NEXT_SELECT_ID.fetch_add(1, Ordering::Relaxed)
        )
    });
    let root_class = if open() {
        merged_class("ui-select ui-multiselect open", &class)
    } else {
        merged_class("ui-select ui-multiselect", &class)
    };
    let list_id = format!("{select_id}-list");
    let selected_options = options
        .iter()
        .filter(|option| values.contains(&option.value))
        .cloned()
        .collect::<Vec<_>>();
    let current = if let [selected] = selected_options.as_slice() {
        selected.clone()
    } else if selected_options.is_empty() {
        SelectOption::new("", "No displays selected").icon("icon-monitor")
    } else {
        SelectOption::new("", format!("{} displays", selected_options.len()))
            .detail(
                selected_options
                    .iter()
                    .map(|option| option.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .icon("icon-monitor")
    };

    let sync_select_id = select_id.clone();
    use_effect(move || {
        if open() && open_select().as_deref() != Some(sync_select_id.as_str()) {
            open.set(false);
            menu_style.set(String::new());
        }
    });

    let position_select_id = select_id.clone();
    use_effect(move || {
        let select_id = position_select_id.clone();
        if !open() {
            menu_style.set(String::new());
            return;
        }

        spawn(async move {
            let script = format!(
                r#"
const root = document.querySelector('[data-ui-select-id="{select_id}"]');
const trigger = root?.querySelector('.ui-select-trigger');
const list = root?.querySelector('.ui-select-list');
if (!trigger || !list) return '';

const rect = trigger.getBoundingClientRect();
const gutter = 12;
const gap = 8;
const viewportWidth = window.innerWidth;
const viewportHeight = window.innerHeight;
const width = Math.min(rect.width, viewportWidth - gutter * 2);
const left = Math.min(Math.max(gutter, rect.left), viewportWidth - gutter - width);
const desiredHeight = Math.min(list.scrollHeight, 320);
const spaceAbove = Math.max(0, rect.top - gutter - gap);
const spaceBelow = Math.max(0, viewportHeight - rect.bottom - gutter - gap);
const placeBelow = spaceBelow >= desiredHeight || spaceBelow >= spaceAbove;
const height = Math.max(44, Math.min(desiredHeight, placeBelow ? spaceBelow : spaceAbove));
const top = placeBelow ? rect.bottom + gap : Math.max(gutter, rect.top - gap - height);
const shift = placeBelow ? -6 : 6;
const origin = placeBelow ? 'top center' : 'bottom center';
return `left:${{left}}px;top:${{top}}px;width:${{width}}px;--ui-select-height:${{height}}px;--ui-select-shift:${{shift}}px;--ui-select-origin:${{origin}};`;
"#
            );
            if let Ok(result) = dioxus::document::eval(&script).await
                && let Some(style) = result.as_str()
            {
                menu_style.set(style.to_string());
            }
        });
    });

    let trigger_select_id = select_id.clone();
    let dismiss_select_id = select_id.clone();
    rsx! {
        div { class: "{root_class}", "data-ui-select-id": "{select_id}",
            if open() {
                button {
                    r#type: "button",
                    class: "ui-select-dismiss",
                    tabindex: "-1",
                    aria_hidden: "true",
                    onclick: move |_| {
                        open.set(false);
                        menu_style.set(String::new());
                        if open_select().as_deref() == Some(dismiss_select_id.as_str()) {
                            open_select.set(None);
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "ui-select-trigger",
                role: "combobox",
                aria_label: "{aria_label}",
                aria_haspopup: "listbox",
                aria_controls: "{list_id}",
                aria_expanded: if open() { "true" } else { "false" },
                onclick: move |_| {
                    if open() {
                        open.set(false);
                        menu_style.set(String::new());
                        open_select.set(None);
                    } else {
                        open_select.set(Some(trigger_select_id.clone()));
                        open.set(true);
                    }
                },
                onkeydown: move |event| {
                    match event.data().key().to_string().as_str() {
                        "Escape" if open() => {
                            event.prevent_default();
                            open.set(false);
                            menu_style.set(String::new());
                            open_select.set(None);
                        }
                        "ArrowDown" | "ArrowUp" if !open() => {
                            event.prevent_default();
                            open_select.set(Some(select_id.clone()));
                            open.set(true);
                        }
                        _ => {}
                    }
                },
                div { class: "ui-select-current",
                    if let Some(icon_url) = current.icon_url.as_deref() {
                        span {
                            class: "solar-icon ui-select-current-icon ui-select-preview-icon",
                            style: "{select_option_icon_style(icon_url)}"
                        }
                    } else if let Some(icon_class) = current.icon_class.as_deref() {
                        span { class: "solar-icon ui-select-current-icon {icon_class}" }
                    }
                    div { class: "ui-select-current-copy",
                        span { class: "ui-select-current-label", "{current.label}" }
                        if let Some(detail) = current.detail.as_deref() {
                            span { class: "ui-select-current-detail", "{detail}" }
                        }
                    }
                }
                span { class: "solar-icon ui-select-chevron icon-down" }
            }
            if open() {
                div {
                    class: if menu_style().is_empty() { "ui-select-menu" } else { "ui-select-menu ready" },
                    style: "{menu_style}",
                    div {
                        class: "ui-select-list",
                        id: "{list_id}",
                        role: "listbox",
                        aria_multiselectable: "true",
                        for option in options.iter().cloned() {
                            {
                                let is_selected = values.contains(&option.value);
                                let cannot_remove = required && is_selected && values.len() == 1;
                                let option_value = option.value.clone();
                                let next_values = values.clone();
                                rsx! {
                                    div {
                                        key: "multi-select-option-{option.value}",
                                        class: if is_selected { "ui-select-item selected" } else { "ui-select-item" },
                                        role: "presentation",
                                        button {
                                            r#type: "button",
                                            class: "ui-select-item-button",
                                            role: "option",
                                            aria_selected: is_selected,
                                            aria_disabled: cannot_remove,
                                            title: if cannot_remove { "At least one display must remain selected" } else { "" },
                                            onclick: move |_| {
                                                if cannot_remove {
                                                    return;
                                                }
                                                let mut updated = next_values.clone();
                                                if is_selected {
                                                    updated.retain(|value| value != &option_value);
                                                } else {
                                                    updated.push(option_value.clone());
                                                }
                                                onchange.call(updated);
                                            },
                                            div { class: "ui-select-item-main",
                                                if let Some(icon_url) = option.icon_url.as_deref() {
                                                    span {
                                                        class: "solar-icon ui-select-item-icon ui-select-preview-icon",
                                                        style: "{select_option_icon_style(icon_url)}"
                                                    }
                                                } else if let Some(icon_class) = option.icon_class.as_deref() {
                                                    span { class: "solar-icon ui-select-item-icon {icon_class}" }
                                                }
                                                div { class: "ui-select-item-copy",
                                                    span { class: "ui-select-item-label", "{option.label}" }
                                                    if let Some(detail) = option.detail.as_deref() {
                                                        span { class: "ui-select-item-detail", "{detail}" }
                                                    }
                                                }
                                                span {
                                                    class: if is_selected { "ui-multiselect-check selected" } else { "ui-multiselect-check" },
                                                    aria_hidden: "true",
                                                    "✓"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn merged_class(base: &str, extra: &str) -> String {
    if extra.trim().is_empty() {
        base.to_string()
    } else {
        format!("{base} {extra}")
    }
}

fn parse_select_height(style: &str) -> Option<f64> {
    let marker = "--ui-select-height:";
    let start = style.find(marker)? + marker.len();
    let rest = &style[start..];
    let end = rest.find("px")?;
    rest[..end].trim().parse().ok()
}

fn select_option_label_style(font_family: Option<&str>) -> String {
    let Some(font_family) = font_family
        .map(str::trim)
        .filter(|family| !family.is_empty())
    else {
        return String::new();
    };

    format!(
        "font-family: \"{}\", sans-serif;",
        css_string_value(font_family)
    )
}

fn canonical_hex_color(value: &str) -> Option<String> {
    let digits = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    match digits.len() {
        3 => {
            let mut expanded = String::with_capacity(7);
            expanded.push('#');
            for digit in digits.chars() {
                expanded.push(digit);
                expanded.push(digit);
            }
            Some(expanded.to_ascii_uppercase())
        }
        6 => Some(format!("#{}", digits.to_ascii_uppercase())),
        _ => None,
    }
}

fn hex_to_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let color = canonical_hex_color(value)?;
    Some((
        u8::from_str_radix(&color[1..3], 16).ok()?,
        u8::from_str_radix(&color[3..5], 16).ok()?,
        u8::from_str_radix(&color[5..7], 16).ok()?,
    ))
}

fn color_prefers_dark_foreground(red: u8, green: u8, blue: u8) -> bool {
    let perceived_brightness =
        299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue);
    perceived_brightness >= 150_000
}

fn hex_to_hsv(value: &str) -> Option<(f64, f64, f64)> {
    let (red, green, blue) = hex_to_rgb(value)?;
    let red = f64::from(red) / 255.0;
    let green = f64::from(green) / 255.0;
    let blue = f64::from(blue) / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let hue = if delta <= f64::EPSILON {
        0.0
    } else if (maximum - red).abs() <= f64::EPSILON {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if (maximum - green).abs() <= f64::EPSILON {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let saturation = if maximum <= f64::EPSILON {
        0.0
    } else {
        delta / maximum
    };
    Some((hue, saturation * 100.0, maximum * 100.0))
}

fn hsv_to_hex(hue: f64, saturation: f64, brightness: f64) -> String {
    let hue = hue.rem_euclid(360.0);
    let saturation = (saturation / 100.0).clamp(0.0, 1.0);
    let brightness = (brightness / 100.0).clamp(0.0, 1.0);
    let chroma = brightness * saturation;
    let sector = hue / 60.0;
    let secondary = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector.floor() as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let match_value = brightness - chroma;
    format!(
        "#{:02X}{:02X}{:02X}",
        ((red + match_value) * 255.0).round() as u8,
        ((green + match_value) * 255.0).round() as u8,
        ((blue + match_value) * 255.0).round() as u8,
    )
}

fn sync_hsv_signals(
    color: &str,
    mut hue: Signal<f64>,
    mut saturation: Signal<f64>,
    mut brightness: Signal<f64>,
) {
    if let Some((next_hue, next_saturation, next_brightness)) = hex_to_hsv(color) {
        hue.set(next_hue);
        saturation.set(next_saturation);
        brightness.set(next_brightness);
    }
}

fn preview_spectrum_color(
    pointer_x: f64,
    pointer_y: f64,
    (width, height): (f64, f64),
    hue: f64,
    mut saturation: Signal<f64>,
    mut brightness: Signal<f64>,
    mut draft: Signal<String>,
) -> Option<String> {
    let (next_saturation, next_brightness) =
        spectrum_position_to_sv(pointer_x, pointer_y, width, height)?;
    let color = hsv_to_hex(hue, next_saturation, next_brightness);
    saturation.set(next_saturation);
    brightness.set(next_brightness);
    draft.set(color.clone());
    Some(color)
}

fn spectrum_position_to_sv(
    pointer_x: f64,
    pointer_y: f64,
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    Some((
        (pointer_x / width * 100.0).clamp(0.0, 100.0),
        (100.0 - pointer_y / height * 100.0).clamp(0.0, 100.0),
    ))
}

fn commit_picker_color(
    color: String,
    mut last_committed: Signal<String>,
    onchange: EventHandler<String>,
) {
    if last_committed.peek().eq_ignore_ascii_case(&color) {
        return;
    }
    last_committed.set(color.clone());
    onchange.call(color);
}

fn select_option_has_icon(option: &SelectOption) -> bool {
    option.icon_class.is_some() || option.icon_url.is_some()
}

fn select_option_icon_style(icon_url: &str) -> String {
    format!("--icon: url(\"{}\");", css_string_value(icon_url))
}

fn css_string_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' | '\r' | '\u{000c}' => escaped.push(' '),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod color_tests {
    use super::{
        canonical_hex_color, color_prefers_dark_foreground, hex_to_hsv, hsv_to_hex,
        spectrum_position_to_sv,
    };

    #[test]
    fn exact_hex_colors_are_canonicalized_without_changing_their_value() {
        assert_eq!(canonical_hex_color("#7D42FB").as_deref(), Some("#7D42FB"));
        assert_eq!(canonical_hex_color("7d42fb").as_deref(), Some("#7D42FB"));
        assert_eq!(canonical_hex_color("#abc").as_deref(), Some("#AABBCC"));
        assert_eq!(canonical_hex_color("#12FG00"), None);
    }

    #[test]
    fn hsv_conversion_round_trips_requested_colors() {
        for color in ["#7D42FB", "#131313", "#FFFFFF", "#28A745"] {
            let (hue, saturation, brightness) = hex_to_hsv(color).unwrap();
            assert_eq!(hsv_to_hex(hue, saturation, brightness), color);
        }
    }

    #[test]
    fn palette_icon_contrast_tracks_the_swatch_brightness() {
        assert!(color_prefers_dark_foreground(255, 255, 255));
        assert!(color_prefers_dark_foreground(243, 156, 25));
        assert!(!color_prefers_dark_foreground(34, 47, 61));
        assert!(!color_prefers_dark_foreground(126, 64, 253));
    }

    #[test]
    fn spectrum_pointer_coordinates_map_to_saturation_and_brightness() {
        assert_eq!(
            spectrum_position_to_sv(125.0, 25.0, 250.0, 100.0),
            Some((50.0, 75.0))
        );
        assert_eq!(
            spectrum_position_to_sv(-20.0, 140.0, 250.0, 100.0),
            Some((0.0, 0.0))
        );
        assert_eq!(spectrum_position_to_sv(0.0, 0.0, 0.0, 100.0), None);
    }
}
