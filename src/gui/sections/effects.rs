use dioxus::prelude::*;

use super::general::Toggle;
use crate::gui::controls::{Select, SelectOption};

pub(crate) fn render() -> Element {
    let mut selected_device = use_signal(String::new);
    let mut refresh = use_signal(|| 0_u64);
    let mut effects = use_signal(|| crate::read_apo_effects().unwrap_or_default());
    let mut save_status = use_signal(String::new);
    let mut action_running = use_signal(|| false);
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
    let saved_state = crate::apo_test_has_saved_state();
    let can_install = crate::apo_test_assets_available();
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
                    if (can_install && !linked) || saved_state {
                        if let Some(device) = selected {
                            button {
                                r#type: "button",
                                class: "secondary",
                                disabled: action_running(),
                                onclick: {
                                    let device_id = device.id.clone();
                                    move |_| {
                                        action_running.set(true);
                                        save_status.set("Waiting for Windows administrator approval…".into());
                                        let device_id = device_id.clone();
                                        let (sender, receiver) = std::sync::mpsc::channel();
                                        std::thread::spawn(move || {
                                            let _ = sender.send(crate::run_apo_test_action(!saved_state, &device_id));
                                        });
                                        spawn(async move {
                                            let result = loop {
                                                match receiver.try_recv() {
                                                    Ok(result) => break result,
                                                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                                        break Err(anyhow::anyhow!("APO action did not return a result"));
                                                    }
                                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                                    }
                                                }
                                            };
                                            save_status.set(match result {
                                                Ok(()) => "Done. Reopen the microphone app to apply the change.".into(),
                                                Err(error) => format!("APO action failed: {error}"),
                                            });
                                            if let Ok(saved) = crate::read_apo_effects() {
                                                effects.set(saved);
                                            }
                                            action_running.set(false);
                                            refresh += 1;
                                        });
                                    }
                                },
                                if saved_state { "Restore previous effect" } else { "Activate MuteGuard APO" }
                            }
                        }
                    }
                    if linked && !saved_state {
                        p { "MuteGuard APO is linked, but no restoration backup was found. The app will not overwrite this association." }
                    }
                    p { role: "status", "{save_status}" }
                }
            }
            section { class: "sound-card",
                h2 { "Effects" }
                if linked {
                    div { class: "sound-card-title startup-row",
                        div { class: "startup-copy",
                            h3 { "RNNoise" }
                            p { "Reduces background noise. Requires a 48 kHz recording format; adds about 20 ms of latency." }
                        }
                        Toggle {
                            checked: current_effects.noise,
                            onchange: move |checked| {
                                let next = crate::ApoEffects { noise: checked, ..effects() };
                                match crate::write_apo_effects(next) {
                                    Ok(()) => { effects.set(next); save_status.set("Saved. Reopen the microphone app to apply.".into()); }
                                    Err(error) => save_status.set(format!("Could not save effects: {error}")),
                                }
                            }
                        }
                    }
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
                    p { "At other sample rates RNNoise is bypassed; gate and compressor still work." }
                } else {
                    p { "Link the MuteGuard APO to this microphone before changing effects." }
                }
            }
        }
    }
}
