use dioxus::prelude::*;

use super::general::Toggle;
use crate::gui::controls::{Select, SelectOption};

pub(crate) fn render() -> Element {
    let mut selected_device = use_signal(String::new);
    let mut refresh = use_signal(|| 0_u64);
    let mut effects = use_signal(|| crate::read_apo_effects().unwrap_or_default());
    let mut save_status = use_signal(String::new);
    let _ = refresh();
    let devices = crate::active_capture_devices();
    let selected = selected_device();
    let selected = devices
        .iter()
        .find(|device| device.id == selected)
        .or_else(|| devices.first());
    let status = selected
        .map(|device| crate::capture_effect_status(&device.id))
        .unwrap_or_default();
    let linked = status == "MuteGuard prototype linked";
    let current_effects = effects();
    let options = devices
        .iter()
        .map(|device| SelectOption::new(device.id.clone(), device.name.clone()))
        .collect();

    rsx! {
        section { class: "general-panel", id: "microphone-effects",
            div { class: "general-header section-head-row",
                div { class: "startup-copy",
                    h1 { "Microphone effects" }
                    p { "Choose a microphone and configure its MuteGuard audio effects." }
                }
                button {
                    r#type: "button",
                    class: "secondary",
                    onclick: move |_| {
                        refresh += 1;
                        if let Ok(saved) = crate::read_apo_effects() {
                            effects.set(saved);
                        }
                    },
                    "Refresh"
                }
            }
            section { class: "sound-card",
                h2 { "Microphone" }
                if devices.is_empty() {
                    p { "No active microphone found." }
                } else {
                    Select {
                        aria_label: "Microphone for audio effects".to_string(),
                        value: selected.map(|device| device.id.clone()).unwrap_or_default(),
                        options,
                        searchable: true,
                        onchange: move |id: String| selected_device.set(id),
                    }
                    p { role: "status", "{status}" }
                    p { "This is the Windows endpoint association. It does not confirm that an app is using the effect." }
                }
            }
            section { class: "sound-card",
                h2 { "Effects" }
                if linked {
                    div { class: "sound-card-title startup-row",
                        div { class: "startup-copy",
                            h3 { "Noise gate" }
                            p { "Reduces sound below -45 dB; 100 ms hold and 150 ms release." }
                        }
                        Toggle {
                            checked: current_effects.gate,
                            onchange: move |checked| {
                                let next = crate::ApoEffects { gate: checked, ..effects() };
                                match crate::write_apo_effects(next) {
                                    Ok(()) => { effects.set(next); save_status.set("Saved. Reopen the microphone app to apply.".into()); }
                                    Err(error) => save_status.set(format!("Could not save effects: {error}")),
                                }
                            }
                        }
                    }
                    div { class: "sound-card-title startup-row",
                        div { class: "startup-copy",
                            h3 { "Compressor" }
                            p { "-18 dB threshold, 2.5:1 ratio; 10 ms attack and 150 ms release." }
                        }
                        Toggle {
                            checked: current_effects.compressor,
                            onchange: move |checked| {
                                let next = crate::ApoEffects { compressor: checked, ..effects() };
                                match crate::write_apo_effects(next) {
                                    Ok(()) => { effects.set(next); save_status.set("Saved. Reopen the microphone app to apply.".into()); }
                                    Err(error) => save_status.set(format!("Could not save effects: {error}")),
                                }
                            }
                        }
                    }
                    p { role: "status", "{save_status}" }
                    p { "RNNoise is not available in this prototype yet." }
                } else {
                    p { "Link the MuteGuard APO to this microphone before changing effects." }
                }
            }
        }
    }
}
