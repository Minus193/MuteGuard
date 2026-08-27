use std::time::Duration;

use dioxus::prelude::*;

use crate::gui::controls::{Select, SelectOption};

pub(crate) fn render(settings: Signal<super::super::SettingsSnapshot>) -> Element {
    let mut recording_id = use_signal(|| None::<String>);
    let mut pending_binding = use_signal(|| None::<crate::HotkeyBinding>);
    let snapshot = settings();
    let mut bindings = snapshot.config.hotkeys.to_vec();
    if let Some(binding) = pending_binding() {
        bindings.push(binding);
    }
    let mut target_options = vec![
        SelectOption::new("", "Default communications microphone")
            .icon_url(crate::overlay_icons::overlay_icon_css_url("fluent", false)),
        SelectOption::new(crate::HOTKEY_TARGET_ALL_MICROPHONES, "All microphones")
            .icon("icon-widget"),
    ];
    for device in crate::active_capture_devices() {
        target_options.push(
            SelectOption::new(device.id, device.name)
                .detail("Specific device")
                .icon_url(crate::overlay_icons::overlay_icon_css_url("fluent", false)),
        );
    }
    for target in bindings
        .iter()
        .filter_map(|binding| binding.target.as_deref())
    {
        if target != crate::HOTKEY_TARGET_ALL_MICROPHONES
            && !target_options.iter().any(|option| option.value == target)
        {
            target_options.push(
                SelectOption::new(target, "Unavailable microphone")
                    .detail("Previously selected device")
                    .icon_url(crate::overlay_icons::overlay_icon_css_url("fluent", false)),
            );
        }
    }

    use_drop(|| crate::set_settings_hotkey_recording(false));

    use_effect(move || {
        let Some(id) = recording_id() else {
            crate::set_settings_hotkey_recording(false);
            return;
        };

        crate::set_settings_hotkey_recording(true);
        spawn(async move {
            loop {
                if recording_id().as_deref() != Some(id.as_str()) {
                    break;
                }

                let shortcut = crate::take_settings_pressed_shortcut();
                if let Some(shortcut) = shortcut {
                    let _ = apply_recorded_shortcut(settings, pending_binding, &id, shortcut);
                    recording_id.set(None);
                    crate::set_settings_hotkey_recording(false);
                    break;
                }

                tokio::time::sleep(Duration::from_millis(12)).await;
            }
        });
    });

    rsx! {
        section {
            class: "hotkeys-panel",
            id: "hotkeys-overview",
            div { class: "hotkeys-header section-head-row",
                div {
                    h1 { "Hotkeys" }
                    p { "Each shortcut toggles either the default microphone or every active microphone." }
                }
                button {
                    class: "secondary add-hotkey-button",
                    onclick: move |_| {
                        let binding = crate::HotkeyBinding {
                            shortcut: crate::Shortcut::empty(),
                            ..crate::HotkeyBinding::default()
                        };
                        let id = binding.id.clone();
                        pending_binding.set(Some(binding));
                        crate::set_settings_hotkey_recording(false);
                        crate::set_settings_hotkey_recording(true);
                        recording_id.set(Some(id));
                    },
                    span { class: "solar-icon button-icon icon-plus" }
                    "Add hotkey"
                }
            }

            if bindings.is_empty() {
                div { class: "hotkey-empty",
                    span { class: "solar-icon icon-keyboard" }
                    p { "No microphone hotkeys configured." }
                }
            }

            div { class: "hotkey-list settings-card-grid",
                for binding in bindings {
                    HotkeyCard {
                        key: "{binding.id}",
                        settings,
                        binding,
                        target_options: target_options.clone(),
                        recording_id,
                        pending_binding,
                    }
                }
            }
        }
    }
}

#[component]
fn HotkeyCard(
    settings: Signal<super::super::SettingsSnapshot>,
    binding: crate::HotkeyBinding,
    target_options: Vec<SelectOption>,
    mut recording_id: Signal<Option<String>>,
    mut pending_binding: Signal<Option<crate::HotkeyBinding>>,
) -> Element {
    let id = binding.id.clone();
    let target = binding.target.clone().unwrap_or_default();
    let is_recording = recording_id().as_deref() == Some(id.as_str());
    let is_pending = pending_binding()
        .as_ref()
        .is_some_and(|binding| binding.id == id);

    rsx! {
        article { class: "hotkey-card",
            div { class: "hotkey-card-main",
                div { class: "hotkey-card-copy",
                    span { class: "hotkey-card-kicker", "Toggle microphone mute" }
                    strong { class: if is_recording { "shortcut-display recording" } else { "shortcut-display" },
                        if is_recording {
                            "Press and release any key combination or mouse chord…"
                        } else {
                            "{binding.shortcut.display()}"
                        }
                    }
                }
                div { class: "hotkey-card-actions",
                    button {
                        class: if is_recording { "secondary active" } else { "secondary" },
                        aria_pressed: is_recording,
                        onclick: {
                            let id = id.clone();
                            move |_| {
                                if recording_id().as_deref() == Some(id.as_str()) {
                                    cancel_recording(recording_id, pending_binding);
                                } else {
                                    if pending_binding()
                                        .as_ref()
                                        .is_some_and(|binding| binding.id != id)
                                    {
                                        pending_binding.set(None);
                                    }
                                    crate::set_settings_hotkey_recording(false);
                                    crate::set_settings_hotkey_recording(true);
                                    recording_id.set(Some(id.clone()));
                                }
                            }
                        },
                        if is_recording { "Cancel" } else { "Record" }
                    }
                    button {
                        class: "icon-button",
                        title: "Delete hotkey",
                        aria_label: "Delete hotkey",
                        onclick: {
                            let id = id.clone();
                            move |_| {
                                if recording_id().as_deref() == Some(id.as_str()) {
                                    recording_id.set(None);
                                    crate::set_settings_hotkey_recording(false);
                                }
                                if pending_binding()
                                    .as_ref()
                                    .is_some_and(|binding| binding.id == id)
                                {
                                    pending_binding.set(None);
                                    return;
                                }
                                let removed_id = id.clone();
                                super::super::update_settings(settings, move |config| {
                                    config.hotkeys.retain(|item| item.id != removed_id);
                                });
                            }
                        },
                        span { class: "solar-icon icon-trash" }
                    }
                }
            }

            div { class: "hotkey-card-options",
                div { class: "hotkey-field",
                    span { "Target" }
                    Select {
                        aria_label: "Hotkey microphone target".to_string(),
                        value: target,
                        options: target_options,
                        disabled: is_pending,
                        show_current_detail: false,
                        searchable: true,
                        onchange: {
                            let id = id.clone();
                            move |value: String| {
                                let binding_id = id.clone();
                                super::super::update_settings(settings, move |config| {
                                    if let Some(item) = config.hotkeys.iter_mut().find(|item| item.id == binding_id) {
                                        item.target = (!value.is_empty()).then_some(value);
                                    }
                                });
                            }
                        }
                    }
                }
                label { class: "hotkey-ignore-modifiers",
                    input {
                        r#type: "checkbox",
                        checked: binding.ignore_modifiers,
                        disabled: is_pending,
                        onchange: move |event| {
                            let checked = event.checked();
                            let binding_id = id.clone();
                            super::super::update_settings(settings, move |config| {
                                if let Some(item) = config.hotkeys.iter_mut().find(|item| item.id == binding_id) {
                                    item.ignore_modifiers = checked;
                                }
                            });
                        }
                    }
                    span { "Ignore modifiers" }
                }
            }
        }
    }
}

fn apply_recorded_shortcut(
    settings: Signal<super::super::SettingsSnapshot>,
    mut pending_binding: Signal<Option<crate::HotkeyBinding>>,
    id: &str,
    shortcut: crate::Shortcut,
) -> bool {
    if pending_binding()
        .as_ref()
        .is_some_and(|binding| binding.id == id)
    {
        let Some(mut binding) = pending_binding() else {
            return false;
        };
        binding.shortcut = shortcut;
        if super::super::update_settings(settings, move |config| config.hotkeys.push(binding)) {
            pending_binding.set(None);
            return true;
        }
        return false;
    }

    let id = id.to_string();
    super::super::update_settings(settings, move |config| {
        if let Some(binding) = config.hotkeys.iter_mut().find(|binding| binding.id == id) {
            binding.shortcut = shortcut;
        }
    })
}

fn cancel_recording(
    mut recording_id: Signal<Option<String>>,
    mut pending_binding: Signal<Option<crate::HotkeyBinding>>,
) {
    let recording_id_value = recording_id.peek().clone();
    if pending_binding
        .peek()
        .as_ref()
        .zip(recording_id_value.as_ref())
        .is_some_and(|(binding, id)| binding.id == *id)
    {
        pending_binding.set(None);
    }
    recording_id.set(None);
    crate::set_settings_hotkey_recording(false);
}
