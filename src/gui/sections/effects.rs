use dioxus::prelude::*;

use crate::gui::controls::{Select, SelectOption};

pub(crate) fn render() -> Element {
    let mut selected_device = use_signal(String::new);
    let mut refresh = use_signal(|| 0_u64);
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
    let options = devices
        .iter()
        .map(|device| SelectOption::new(device.id.clone(), device.name.clone()))
        .collect();

    rsx! {
        section { class: "general-panel", id: "microphone-effects",
            div { class: "general-header section-head-row",
                div { class: "startup-copy",
                    h1 { "Microphone effects" }
                    p { "See which stream effect Windows has linked to a microphone." }
                }
                button {
                    r#type: "button",
                    class: "secondary",
                    onclick: move |_| refresh += 1,
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
                p { "The current MuteGuard prototype only applies a fixed 50% level. Noise suppression, gate, and compression are not configurable yet." }
            }
        }
    }
}
