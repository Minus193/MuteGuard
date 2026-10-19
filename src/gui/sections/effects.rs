use dioxus::prelude::*;

use super::general::Toggle;
use crate::gui::controls::{Select, SelectOption};

fn save_effects(
    next: crate::ApoEffects,
    mut effects: Signal<crate::ApoEffects>,
    mut save_status: Signal<String>,
) {
    let next = next.clamped();
    match crate::write_apo_effects(next) {
        Ok(()) => {
            effects.set(next);
            save_status.set("Saved. Reopen the microphone app to apply.".into());
        }
        Err(error) => save_status.set(format!("Could not save effects: {error}")),
    }
}

fn run_apo_action(
    install: bool,
    device_id: String,
    mut action_running: Signal<bool>,
    mut save_status: Signal<String>,
    mut effects: Signal<crate::ApoEffects>,
    mut refresh: Signal<u64>,
) {
    action_running.set(true);
    save_status.set("Waiting for Windows administrator approval...".into());
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(crate::run_apo_test_action(install, &device_id));
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
                        if let Ok(number) = event.value().replace(',', ".").parse::<f32>()
                            && number.is_finite()
                        {
                            onchange.call(((number * scale as f32).round() as i32).clamp(min, max));
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
    let save_status = use_signal(String::new);
    let action_running = use_signal(|| false);
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
    let linked = status == "MuteGuard APO active";
    let saved_state = crate::apo_test_has_saved_state();
    let can_install = crate::apo_test_assets_available();
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
                                    move |_| run_apo_action(!saved_state, device_id.clone(), action_running, save_status, effects, refresh)
                                },
                                if saved_state { "Deactivate MuteGuard APO" } else { "Activate MuteGuard APO" }
                            }
                }
                if linked && !saved_state {
                    p { "MuteGuard APO is linked, but no activation record was found. The app will not change this association." }
                }
                p { role: "status", "{save_status}" }
            }
            div { class: "effect-workspace",
                super::analyzer::Analyzer { device_id: selected.map(|device| device.id.clone()).unwrap_or_default(), effects }
                EffectModules { effects, save_status, linked }
            }

        }
    }
}

#[component]
fn EffectModules(
    effects: Signal<crate::ApoEffects>,
    save_status: Signal<String>,
    linked: bool,
) -> Element {
    let current_effects = effects();
    rsx! {
        section { class: "effect-rack",
            div { class: "effect-rack-heading",
                span { class: "effect-eyebrow", "YOUR SIGNAL CHAIN" }
                h2 { "Effects rack" }
                p { "Clean up → shape your voice → control peaks. Audio flows from top to bottom." }
            }
            if linked {
                for (index, module) in current_effects.order.into_iter().enumerate() {
                    EffectCard { key: "{module:?}", module, index, effects, save_status }
                }
                p { "RNNoise needs 48 kHz and adds about 20 ms of latency. The other modules work at other sample rates." }
                p { "Keep the limiter last to protect the final output. Changes apply when you reopen the recording app." }
            } else {
                p { "Link the MuteGuard APO to this microphone before changing effects." }
            }
        }
    }
}

pub(super) fn module_info(
    module: crate::EffectModule,
    current: crate::ApoEffects,
) -> (&'static str, &'static str, bool) {
    use crate::EffectModule::*;
    match module {
        HighPass => (
            "High-pass",
            "Cuts low-frequency rumble. Choose a 12 or 24 dB/octave slope.",
            current.high_pass,
        ),
        Noise => (
            "RNNoise",
            "Reduces background noise while you speak.",
            current.noise,
        ),
        Gate => (
            "Noise gate",
            "Silences quieter sounds between words. Usually use the gate or expander, rather than both.",
            current.gate,
        ),
        Expander => (
            "Downward expander",
            "Gradually reduces quiet sounds, with a softer transition than the gate.",
            current.expander,
        ),
        DeEsser => (
            "De-esser",
            "Reduces harsh S and Sh sounds around the selected frequency.",
            current.de_esser,
        ),
        Eq => (
            "EQ",
            "Three bell bands. Higher Q affects a narrower frequency range. Starts flat.",
            current.eq,
        ),
        Compressor => (
            "Compressor",
            "Reduces loud passages to even out your voice.",
            current.compressor,
        ),
        Limiter => (
            "Limiter",
            "Caps sample peaks at the ceiling. Cannot repair distortion already recorded by the microphone.",
            current.limiter,
        ),
    }
}

fn set_module_enabled(current: &mut crate::ApoEffects, module: crate::EffectModule, enabled: bool) {
    use crate::EffectModule::*;
    match module {
        HighPass => current.high_pass = enabled,
        Noise => current.noise = enabled,
        Gate => current.gate = enabled,
        Expander => current.expander = enabled,
        DeEsser => current.de_esser = enabled,
        Eq => current.eq = enabled,
        Compressor => current.compressor = enabled,
        Limiter => current.limiter = enabled,
    }
}

fn move_module(
    module: crate::EffectModule,
    up: bool,
    effects: Signal<crate::ApoEffects>,
    save_status: Signal<String>,
) {
    let mut next = effects();
    let Some(index) = next.order.iter().position(|item| *item == module) else {
        return;
    };
    let target = if up {
        index.checked_sub(1)
    } else {
        (index + 1 < next.order.len()).then_some(index + 1)
    };
    if let Some(target) = target {
        next.order.swap(index, target);
        save_effects(next, effects, save_status);
    }
}

struct EffectParameter {
    label: &'static str,
    unit: &'static str,
    value: i32,
    min: i32,
    max: i32,
    step: &'static str,
    scale: i32,
    set: fn(&mut crate::ApoEffects, i32),
}

fn module_parameters(
    module: crate::EffectModule,
    current: crate::ApoEffects,
) -> Vec<EffectParameter> {
    macro_rules! parameter {
        ($field:ident, $label:literal, $unit:literal, $min:expr, $max:expr, $step:literal, $scale:expr) => {
            EffectParameter {
                label: $label,
                unit: $unit,
                value: current.$field,
                min: $min,
                max: $max,
                step: $step,
                scale: $scale,
                set: |next, value| next.$field = value,
            }
        };
    }
    use crate::EffectModule::*;
    match module {
        HighPass => vec![
            parameter!(high_pass_hz, "Cutoff", "Hz", 40, 300, "5", 1),
            parameter!(high_pass_slope, "Slope", "dB/oct", 12, 24, "12", 1),
        ],
        Noise => vec![parameter!(noise_strength, "Strength", "%", 0, 100, "1", 1)],
        Gate => vec![
            parameter!(gate_threshold_db, "Threshold", "dB", -80, -10, "1", 1),
            parameter!(gate_hold_ms, "Hold", "ms", 0, 500, "10", 1),
            parameter!(gate_release_ms, "Release", "ms", 10, 1000, "10", 1),
        ],
        Expander => vec![
            parameter!(expander_threshold_db, "Threshold", "dB", -80, -10, "1", 1),
            parameter!(expander_ratio_tenths, "Ratio", ":1", 10, 100, "0.1", 10),
            parameter!(expander_attack_ms, "Attack", "ms", 1, 100, "1", 1),
            parameter!(expander_release_ms, "Release", "ms", 10, 1000, "10", 1),
        ],
        DeEsser => vec![
            parameter!(de_esser_hz, "Frequency", "Hz", 3000, 10000, "100", 1),
            parameter!(de_esser_threshold_db, "Threshold", "dB", -60, -5, "1", 1),
            parameter!(de_esser_ratio_tenths, "Ratio", ":1", 10, 100, "0.1", 10),
            parameter!(de_esser_attack_ms, "Attack", "ms", 1, 100, "1", 1),
            parameter!(de_esser_release_ms, "Release", "ms", 10, 1000, "10", 1),
            parameter!(
                de_esser_max_reduction_db,
                "Max reduction",
                "dB",
                1,
                18,
                "1",
                1
            ),
        ],
        Eq => vec![
            parameter!(eq_low_hz, "Low frequency", "Hz", 60, 500, "10", 1),
            parameter!(eq_low_gain_db, "Low gain", "dB", -12, 12, "1", 1),
            parameter!(eq_low_q_tenths, "Low Q", "", 3, 100, "0.1", 10),
            parameter!(eq_mid_hz, "Mid frequency", "Hz", 500, 6000, "100", 1),
            parameter!(eq_mid_gain_db, "Mid gain", "dB", -12, 12, "1", 1),
            parameter!(eq_mid_q_tenths, "Mid Q", "", 3, 100, "0.1", 10),
            parameter!(eq_high_hz, "High frequency", "Hz", 4000, 12000, "100", 1),
            parameter!(eq_high_gain_db, "High gain", "dB", -12, 12, "1", 1),
            parameter!(eq_high_q_tenths, "High Q", "", 3, 100, "0.1", 10),
        ],
        Compressor => vec![
            parameter!(compressor_threshold_db, "Threshold", "dB", -40, 0, "1", 1),
            parameter!(compressor_ratio_tenths, "Ratio", ":1", 10, 100, "0.1", 10),
            parameter!(compressor_attack_ms, "Attack", "ms", 1, 100, "1", 1),
            parameter!(compressor_release_ms, "Release", "ms", 20, 1000, "10", 1),
        ],
        Limiter => vec![
            parameter!(limiter_threshold_db, "Ceiling", "dB", -20, -1, "1", 1),
            parameter!(limiter_release_ms, "Release", "ms", 10, 1000, "10", 1),
        ],
    }
}

#[component]
fn EffectCard(
    module: crate::EffectModule,
    index: usize,
    effects: Signal<crate::ApoEffects>,
    save_status: Signal<String>,
) -> Element {
    let current = effects();
    let (name, description, enabled) = module_info(module, current);
    let (category, symbol, tip) = match module {
        crate::EffectModule::HighPass => (
            "CLEANUP",
            "↗",
            "Raise the cutoff until rumble fades, then back off if your voice sounds thin.",
        ),
        crate::EffectModule::Noise => (
            "CLEANUP",
            "≈",
            "Start strong; reduce the strength if your voice sounds metallic or loses detail.",
        ),
        crate::EffectModule::Gate => (
            "CLEANUP",
            "⊣",
            "Set the threshold above room noise and below quiet speech. Hold keeps word endings intact; release softens the closing.",
        ),
        crate::EffectModule::Expander => (
            "CLEANUP",
            "↘",
            "Use a low ratio for a natural noise floor. Faster attack opens for speech; slower release preserves tails.",
        ),
        crate::EffectModule::DeEsser => (
            "VOICE DETAIL",
            "∿",
            "Say a phrase with S sounds. Find the harsh frequency, then limit reduction so consonants stay clear.",
        ),
        crate::EffectModule::Eq => (
            "TONE",
            "≋",
            "Low adds body, mid changes presence, high changes brightness. Each band is a bell; Q controls width. Make small gain changes.",
        ),
        crate::EffectModule::Compressor => (
            "DYNAMICS",
            "⇥",
            "Lower the threshold to catch more of your voice. Ratio controls reduction; attack and release control how quickly it reacts.",
        ),
        crate::EffectModule::Limiter => (
            "OUTPUT SAFETY",
            "⊤",
            "Keep this last to catch final peaks. Leave headroom below 0 dBFS; it cannot undo microphone distortion.",
        ),
    };
    let values = module_parameters(module, current);
    let summary = if module == crate::EffectModule::Eq {
        format!(
            "Low {:+} dB · Mid {:+} dB · High {:+} dB",
            current.eq_low_gain_db, current.eq_mid_gain_db, current.eq_high_gain_db
        )
    } else {
        values
            .iter()
            .take(3)
            .map(|p| {
                let number = if p.scale == 1 {
                    p.value.to_string()
                } else {
                    format!("{:.1}", p.value as f32 / p.scale as f32)
                };
                format!("{} {}{}", p.label, number, p.unit)
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };
    rsx! {
        section { class: if enabled { "sound-card effect-module is-enabled" } else { "sound-card effect-module" }, aria_label: name,
            div { class: "effect-module-header",
                span { class: "effect-position", "{index + 1}" }
                div { class: "effect-module-name",
                    span { class: "effect-eyebrow", "{category}" }
                    h3 { span { class: "effect-symbol", aria_hidden: "true", "{symbol}" } "{name}" }
                }
                div { class: "effect-order-actions",
                    button { r#type: "button", class: "secondary effect-move", disabled: index == 0,
                        aria_label: "Move {name} up", title: "Move up",
                        onclick: move |_| move_module(module, true, effects, save_status), "↑" }
                    button { r#type: "button", class: "secondary effect-move", disabled: index == 7,
                        aria_label: "Move {name} down", title: "Move down",
                        onclick: move |_| move_module(module, false, effects, save_status), "↓" }
                }
                Toggle { checked: enabled, aria_label: Some(format!("Enable {name}")), onchange: move |checked| {
                    let mut next = effects();
                    set_module_enabled(&mut next, module, checked);
                    save_effects(next, effects, save_status);
                } }
            }
            p { "{description}" }
            div { class: "effect-overview",
                span { class: if enabled { "effect-state is-enabled" } else { "effect-state" }, if enabled { "Active" } else { "Bypassed" } }
                span { "{summary}" }
            }
            details { class: "effect-adjust",
                summary { "Adjust {name}" }
                p { class: "effect-tip", "{tip}" }
                div { class: if module == crate::EffectModule::Eq { "effect-values effect-eq-values" } else { "effect-values" },
                for parameter in values {
                    EffectValue { label: parameter.label, unit: parameter.unit, value: parameter.value,
                        min: parameter.min, max: parameter.max, step: parameter.step, scale: parameter.scale, disabled: false,
                        onchange: move |value| {
                            let mut next = effects();
                            (parameter.set)(&mut next, value);
                            save_effects(next, effects, save_status);
                        }
                    }
                }
                }
            }
        }
    }
}
