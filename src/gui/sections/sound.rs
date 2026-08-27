use dioxus::html::FileData;
use dioxus::prelude::*;

use crate::{
    FeedbackKind,
    gui::{
        controls::{Range, Select, SelectOption},
        sections::general::Toggle,
    },
};

type Settings = Signal<super::super::SettingsSnapshot>;

pub(crate) fn render(settings: Settings) -> Element {
    let snapshot = settings();
    let sound = snapshot.config.sound_feedback;
    let preview_mute = sound.clone();
    let preview_unmute = sound.clone();

    rsx! {
        section { class: "general-panel sound-panel", id: "sound-settings",
            div { class: "general-header",
                h1 { "Sound" }
                p { "Optional audio confirmation when the microphone state changes." }
            }

            div { class: "settings-card-grid",
                section { class: "sound-card startup-card",
                    div { class: "sound-card-title startup-row",
                        div { class: "startup-copy",
                            h2 { "Sound feedback" }
                            p { "Play a short, distinct tone after mute and unmute changes." }
                        }
                        Toggle {
                            checked: sound.enabled,
                            onchange: move |checked| {
                                super::super::update_settings(settings, |config| {
                                    config.sound_feedback.enabled = checked;
                                });
                            }
                        }
                    }
                }

                section { class: "sound-card sound-detail-card",
                    div { class: "sound-card-title",
                        div { class: "startup-copy",
                            h2 { "Feedback level" }
                            p { "This affects only MuteGuard tones, not the Windows microphone volume." }
                        }
                    }
                    Range {
                        label: "Volume".to_string(),
                        value: sound.volume.to_string(),
                        min: "0".to_string(),
                        max: "100".to_string(),
                        step: "1".to_string(),
                        value_suffix: "%".to_string(),
                        onchange: move |value: String| {
                            if let Ok(volume) = value.parse::<u8>() {
                                super::super::update_settings(settings, |config| {
                                    config.sound_feedback.volume = volume.min(100);
                                });
                            }
                        }
                    }
                }

                {sound_choice_card(
                    settings,
                    FeedbackKind::Mute,
                    "Mute sound",
                    "Played when the microphone becomes muted.",
                    sound.mute_source.clone(),
                )}
                {sound_choice_card(
                    settings,
                    FeedbackKind::Unmute,
                    "Unmute sound",
                    "Played when the microphone becomes active.",
                    sound.unmute_source,
                )}

                section { class: "sound-card sound-preview-card",
                    div { class: "sound-card-title",
                        div { class: "startup-copy",
                            h2 { "Preview" }
                            p { "Custom sounds fall back to the built-in tone until a valid file is saved." }
                        }
                    }
                    div { class: "sound-preview-actions",
                        button {
                            r#type: "button",
                            class: "secondary",
                            onclick: move |_| {
                                crate::preview_sound_feedback(FeedbackKind::Mute, &preview_mute)
                            },
                            span { class: "solar-icon button-icon icon-mic-muted" }
                            "Preview mute"
                        }
                        button {
                            r#type: "button",
                            class: "secondary",
                            onclick: move |_| {
                                crate::preview_sound_feedback(FeedbackKind::Unmute, &preview_unmute)
                            },
                            span { class: "solar-icon button-icon icon-mic" }
                            "Preview unmute"
                        }
                    }
                }
            }
        }
    }
}

fn sound_choice_card(
    settings: Settings,
    kind: FeedbackKind,
    title: &'static str,
    description: &'static str,
    source: String,
) -> Element {
    let custom_available = crate::custom_sound_available(kind);
    let status = if custom_available {
        "Custom WAV saved"
    } else {
        "No custom WAV saved"
    };
    let source_options = vec![
        SelectOption::new("Default", "Built-in tone").icon("icon-speaker"),
        SelectOption::new("Custom", "Custom WAV")
            .detail("16-bit PCM, maximum 5 seconds")
            .icon("icon-record"),
    ];

    rsx! {
        section { class: "sound-card sound-source-card",
            div { class: "sound-card-title",
                div { class: "startup-copy",
                    h2 { "{title}" }
                    p { "{description}" }
                }
            }
            div { class: "overlay-field",
                label { "Source" }
                Select {
                    aria_label: format!("{title} source"),
                    value: source,
                    options: source_options,
                    onchange: move |value| {
                        super::super::update_settings(settings, |config| match kind {
                            FeedbackKind::Mute => config.sound_feedback.mute_source = value,
                            FeedbackKind::Unmute => config.sound_feedback.unmute_source = value,
                        });
                    }
                }
            }
            div { class: "sound-file-row",
                div { class: "sound-file-copy",
                    span { class: "sound-file-status", "{status}" }
                    span { class: "sound-file-help", "Choosing another file replaces the previous custom sound." }
                }
                label { class: "secondary sound-file-button",
                    span { class: "solar-icon button-icon icon-plus" }
                    "Choose WAV"
                    input {
                        r#type: "file",
                        accept: ".wav,audio/wav,audio/x-wav",
                        onchange: move |event| import_custom_sound(settings, kind, event.files()),
                    }
                }
            }
        }
    }
}

fn import_custom_sound(settings: Settings, kind: FeedbackKind, files: Vec<FileData>) {
    let Some(file) = files.into_iter().next() else {
        return;
    };
    if file.size() > crate::MAX_CUSTOM_SOUND_BYTES as u64 {
        super::super::set_settings_error(
            settings,
            "Could not save the custom sound: the selected file exceeds the 12 MB safety limit."
                .to_string(),
        );
        return;
    }
    spawn(async move {
        let result = match file.read_bytes().await {
            Ok(bytes) => crate::save_custom_sound(kind, &bytes),
            Err(error) => Err(anyhow::anyhow!("Could not read the selected WAV: {error}")),
        };
        match result {
            Ok(duration_millis) => {
                if super::super::update_settings(settings, |config| match kind {
                    FeedbackKind::Mute => config.sound_feedback.mute_source = "Custom".to_string(),
                    FeedbackKind::Unmute => {
                        config.sound_feedback.unmute_source = "Custom".to_string()
                    }
                }) {
                    super::super::set_settings_notice(
                        settings,
                        format!(
                            "Custom {} sound saved ({:.2} seconds).",
                            kind.label().to_lowercase(),
                            duration_millis as f64 / 1_000.0
                        ),
                    );
                }
            }
            Err(error) => super::super::set_settings_error(
                settings,
                format!("Could not save the custom sound: {error:#}"),
            ),
        }
    });
}
