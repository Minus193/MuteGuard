use dioxus::prelude::*;

use super::general::Toggle;
use crate::gui::controls::{Select, SelectOption};

fn save_effects(
    next: crate::ApoEffects,
    mut effects: Signal<crate::ApoEffects>,
    mut save_status: Signal<String>,
) {
    match crate::write_apo_effects(next) {
        Ok(()) => {
            effects.set(next);
            save_status.set("Saved. Reopen the microphone app to apply.".into());
        }
        Err(error) => save_status.set(format!("Could not save effects: {error}")),
    }
}

#[component]
fn EffectValue(
    label: &'static str,
    unit: &'static str,
    value: i32,
    min: i32,
    max: i32,
    step: &'static str,
    scale: i32,
    disabled: bool,
    onchange: EventHandler<i32>,
) -> Element {
    let display = |number: i32| {
        if scale == 1 {
            number.to_string()
        } else {
            format!("{:.1}", number as f32 / scale as f32)
        }
    };
    let shown = display(value);
    let low = display(min);
    let high = display(max);
    rsx! {
        label { class: "effect-value",
            span { "{label}" }
            div { class: "effect-value-control",
                input {
                    r#type: "number",
                    value: "{shown}",
                    min: "{low}",
                    max: "{high}",
                    step,
                    disabled,
                    onchange: move |event| {
                        if let Ok(number) = event.value().replace(',', ".").parse::<f32>() {
                            if number.is_finite() {
                                onchange.call(((number * scale as f32).round() as i32).clamp(min, max));
                            }
                        }
                    }
                }
                span { "{unit}" }
            }
        }
    }
}

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
                }
                if saved_state || (can_install && !linked && selected.is_some()) {
                            button {
                                r#type: "button",
                                class: "secondary",
                                disabled: action_running(),
                                onclick: {
                                    let device_id = selected.map(|device| device.id.clone()).unwrap_or_default();
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
                if linked && !saved_state {
                    p { "MuteGuard APO is linked, but no restoration backup was found. The app will not overwrite this association." }
                }
                p { role: "status", "{save_status}" }
            }
            section { class: "sound-card",
                h2 { "Effects" }
                if linked {
                    div { class: "effect-module",
                        div { class: "sound-card-title startup-row",
                            div { class: "startup-copy",
                                h3 { "RNNoise" }
                                p { "Reduces background noise at 48 kHz; adds about 20 ms of latency." }
                            }
                            Toggle {
                                checked: current_effects.noise,
                                onchange: move |checked| save_effects(crate::ApoEffects { noise: checked, ..effects() }, effects, save_status)
                            }
                        }
                        div { class: "effect-values",
                            EffectValue { label: "Strength", unit: "%", value: current_effects.noise_strength,
                                min: 0, max: 100, step: "1", scale: 1, disabled: !current_effects.noise,
                                onchange: move |value| save_effects(crate::ApoEffects { noise_strength: value, ..effects() }, effects, save_status)
                            }
                        }
                    }
                    div { class: "effect-module",
                        div { class: "sound-card-title startup-row",
                            div { class: "startup-copy",
                                h3 { "Noise gate" }
                                p { "Silences quieter sounds between words." }
                            }
                            Toggle {
                                checked: current_effects.gate,
                                onchange: move |checked| save_effects(crate::ApoEffects { gate: checked, ..effects() }, effects, save_status)
                            }
                        }
                        div { class: "effect-values",
                            EffectValue { label: "Threshold", unit: "dB", value: current_effects.gate_threshold_db,
                                min: -80, max: -10, step: "1", scale: 1, disabled: !current_effects.gate,
                                onchange: move |value| save_effects(crate::ApoEffects { gate_threshold_db: value, ..effects() }, effects, save_status)
                            }
                            EffectValue { label: "Hold", unit: "ms", value: current_effects.gate_hold_ms,
                                min: 0, max: 500, step: "10", scale: 1, disabled: !current_effects.gate,
                                onchange: move |value| save_effects(crate::ApoEffects { gate_hold_ms: value, ..effects() }, effects, save_status)
                            }
                            EffectValue { label: "Release", unit: "ms", value: current_effects.gate_release_ms,
                                min: 10, max: 1000, step: "10", scale: 1, disabled: !current_effects.gate,
                                onchange: move |value| save_effects(crate::ApoEffects { gate_release_ms: value, ..effects() }, effects, save_status)
                            }
                        }
                    }
                    div { class: "effect-module",
                        div { class: "sound-card-title startup-row",
                            div { class: "startup-copy",
                                h3 { "Compressor" }
                                p { "Evens out loud and quiet parts of your voice." }
                            }
                            Toggle {
                                checked: current_effects.compressor,
                                onchange: move |checked| save_effects(crate::ApoEffects { compressor: checked, ..effects() }, effects, save_status)
                            }
                        }
                        div { class: "effect-values",
                            EffectValue { label: "Threshold", unit: "dB", value: current_effects.compressor_threshold_db,
                                min: -40, max: 0, step: "1", scale: 1, disabled: !current_effects.compressor,
                                onchange: move |value| save_effects(crate::ApoEffects { compressor_threshold_db: value, ..effects() }, effects, save_status)
                            }
                            EffectValue { label: "Ratio", unit: ":1", value: current_effects.compressor_ratio_tenths,
                                min: 10, max: 100, step: "0.1", scale: 10, disabled: !current_effects.compressor,
                                onchange: move |value| save_effects(crate::ApoEffects { compressor_ratio_tenths: value, ..effects() }, effects, save_status)
                            }
                            EffectValue { label: "Attack", unit: "ms", value: current_effects.compressor_attack_ms,
                                min: 1, max: 100, step: "1", scale: 1, disabled: !current_effects.compressor,
                                onchange: move |value| save_effects(crate::ApoEffects { compressor_attack_ms: value, ..effects() }, effects, save_status)
                            }
                            EffectValue { label: "Release", unit: "ms", value: current_effects.compressor_release_ms,
                                min: 20, max: 1000, step: "10", scale: 1, disabled: !current_effects.compressor,
                                onchange: move |value| save_effects(crate::ApoEffects { compressor_release_ms: value, ..effects() }, effects, save_status)
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
