use dioxus::prelude::*;

use crate::gui::tabs::SettingsTab;

#[derive(Clone, Copy, PartialEq, Eq)]
struct GuideItem {
    label: &'static str,
    description: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct GuideTopic {
    tab: SettingsTab,
    id: &'static str,
    title: &'static str,
    description: &'static str,
    icon: &'static str,
    items: &'static [GuideItem],
}

const GENERAL_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Start with Windows",
        description: "Launch MuteGuard when you sign in.",
    },
    GuideItem {
        label: "Mute microphone on startup",
        description: "Mute the default microphone once when MuteGuard starts.",
    },
    GuideItem {
        label: "Microphone change notifications",
        description: "Notify when the default microphone changes, disconnects or reconnects.",
    },
    GuideItem {
        label: "Check for updates",
        description: "Check GitHub for updates once a day. No account is needed.",
    },
    GuideItem {
        label: "Check now / Download update",
        description: "Check now checks immediately. Download update opens the installer in your browser.",
    },
    GuideItem {
        label: "Accent source",
        description: "Follow the Windows accent or choose a custom color.",
    },
    GuideItem {
        label: "Accent color",
        description: "Pick a preset or enter a HEX color.",
    },
];

const HOTKEY_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Add hotkey",
        description: "Press and release a keyboard or mouse shortcut to save it.",
    },
    GuideItem {
        label: "Record / Cancel",
        description: "Record replaces the shortcut. Cancel keeps the previous one.",
    },
    GuideItem {
        label: "Action",
        description: "Toggle mute switches state. Mute and Unmute set a specific state.",
    },
    GuideItem {
        label: "Microphone",
        description: "Choose the microphone to control.",
    },
    GuideItem {
        label: "Specific devices",
        description: "A saved microphone selection works again when the device reconnects.",
    },
    GuideItem {
        label: "All microphones",
        description: "Control every connected microphone. Toggle mute follows the default microphone.",
    },
    GuideItem {
        label: "Ignore modifiers",
        description: "Allow extra Ctrl, Alt, Shift or Windows keys while pressing the shortcut.",
    },
    GuideItem {
        label: "Delete",
        description: "Remove this shortcut.",
    },
];

const OVERLAY_BEHAVIOR_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Overlay switch",
        description: "Show or hide the overlay. It does not block clicks.",
    },
    GuideItem {
        label: "Visibility",
        description: "Show always, only when muted or unmuted, or briefly after a change.",
    },
    GuideItem {
        label: "Temporary duration",
        description: "How long the overlay appears after a change: 0.5 to 10 seconds.",
    },
    GuideItem {
        label: "Monitor",
        description: "Choose the displays where the overlay appears.",
    },
    GuideItem {
        label: "Position",
        description: "Choose a position on each selected display.",
    },
    GuideItem {
        label: "Preview",
        description: "Show the overlay while adjusting it. Stops when you leave this section.",
    },
];

const OVERLAY_CONTENT_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Style",
        description: "Show an icon, text, both, or a status dot.",
    },
    GuideItem {
        label: "Icon family",
        description: "Choose a microphone icon. More shows all styles.",
    },
    GuideItem {
        label: "Icon color",
        description: "Use a custom color, a neutral tone, or the Windows accent.",
    },
    GuideItem {
        label: "Color",
        description: "Pick a preset or enter a HEX color.",
    },
    GuideItem {
        label: "Muted / Unmuted label",
        description: "Choose the text shown for each microphone state.",
    },
    GuideItem {
        label: "Font",
        description: "Choose an installed font for overlay text.",
    },
    GuideItem {
        label: "Font weight",
        description: "Adjust text thickness.",
    },
    GuideItem {
        label: "Scale",
        description: "Resize the whole overlay.",
    },
    GuideItem {
        label: "Content opacity",
        description: "Adjust icon and text transparency independently of the background.",
    },
];

const OVERLAY_BACKGROUND_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Background style",
        description: "Choose a dark, light or transparent background.",
    },
    GuideItem {
        label: "Background opacity",
        description: "Adjust background transparency.",
    },
    GuideItem {
        label: "Corner radius",
        description: "Adjust how rounded the corners are.",
    },
    GuideItem {
        label: "Show border",
        description: "Add an outline around the overlay.",
    },
    GuideItem {
        label: "Border color",
        description: "Choose the outline color.",
    },
];

const TRAY_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Tray icon style",
        description: "Show the app logo, microphone status, or a colored dot.",
    },
    GuideItem {
        label: "Icon family",
        description: "Choose a microphone icon. More shows all styles.",
    },
    GuideItem {
        label: "Icon color",
        description: "Use a custom color, a neutral tone, or the Windows accent.",
    },
    GuideItem {
        label: "Color",
        description: "Pick a preset or enter a HEX color.",
    },
    GuideItem {
        label: "Tray menu",
        description: "Left-click for Settings. Right-click for mute controls and Exit.",
    },
];

const SOUND_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Sound feedback",
        description: "Play a tone when you mute or unmute.",
    },
    GuideItem {
        label: "Volume",
        description: "Adjust feedback tone volume without changing the microphone level.",
    },
    GuideItem {
        label: "Mute / Unmute source",
        description: "Use the built-in tone or a custom WAV for each state.",
    },
    GuideItem {
        label: "Choose WAV",
        description: "Choose a 16-bit PCM WAV up to 5 seconds and 12 MB.",
    },
    GuideItem {
        label: "Missing custom files",
        description: "The built-in tone plays if the custom file is unavailable.",
    },
    GuideItem {
        label: "Preview mute / unmute",
        description: "Listen to the selected sound without changing microphone state.",
    },
];

const EFFECTS_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Enable / Disable effects",
        description: "Apply effects to the selected microphone. Windows may request administrator access.",
    },
    GuideItem {
        label: "Effects",
        description: "Toggle modules, edit values and reorder with the arrows. Keep the limiter last. RNNoise requires 48 kHz.",
    },
    GuideItem {
        label: "Start preview",
        description: "Compare audio before and after effects. Changes apply here immediately; reopen other recording apps.",
    },
    GuideItem {
        label: "Listen",
        description: "Hear processed audio through the default output. Use headphones. Leaving this section stops listening.",
    },
];

const DIAGNOSTICS_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Refresh",
        description: "Update the displayed status without changing settings.",
    },
    GuideItem {
        label: "Application",
        description: "Version, startup and background app status.",
    },
    GuideItem {
        label: "Windows",
        description: "Windows version and required components.",
    },
    GuideItem {
        label: "Audio",
        description: "Microphone, audio connection and sound status.",
    },
    GuideItem {
        label: "Input and overlay",
        description: "Configured shortcuts, overlay and connected displays.",
    },
    GuideItem {
        label: "Updates",
        description: "Latest release, last check and connection errors.",
    },
    GuideItem {
        label: "Copy diagnostics",
        description: "Copy a support report without personal file paths.",
    },
];

const GUIDE_TOPICS: &[GuideTopic] = &[
    GuideTopic {
        tab: SettingsTab::General,
        id: "guide-general",
        title: "General",
        description: "Startup, notifications, updates and color.",
        icon: "icon-settings",
        items: GENERAL_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Hotkeys,
        id: "guide-hotkeys",
        title: "Hotkeys",
        description: "Shortcuts, actions and microphone selection.",
        icon: "icon-keyboard",
        items: HOTKEY_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Overlay,
        id: "guide-overlay-behavior",
        title: "Overlay - behavior",
        description: "When and where the overlay appears.",
        icon: "icon-monitor",
        items: OVERLAY_BEHAVIOR_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Overlay,
        id: "guide-overlay-content",
        title: "Overlay - content",
        description: "Icon, text, size and color.",
        icon: "icon-mic",
        items: OVERLAY_CONTENT_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Overlay,
        id: "guide-overlay-background",
        title: "Overlay - background",
        description: "Background, transparency and border.",
        icon: "icon-contrast",
        items: OVERLAY_BACKGROUND_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Tray,
        id: "guide-tray",
        title: "Tray",
        description: "Tray icon and menu.",
        icon: "icon-widget",
        items: TRAY_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Sound,
        id: "guide-sound",
        title: "Sound",
        description: "Feedback tones and volume.",
        icon: "icon-speaker",
        items: SOUND_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Effects,
        id: "guide-effects",
        title: "Microphone effects",
        description: "Processing, preview and listening.",
        icon: "icon-mic",
        items: EFFECTS_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Diagnostics,
        id: "guide-diagnostics",
        title: "Diagnostics",
        description: "System status for troubleshooting.",
        icon: "icon-diagnostics",
        items: DIAGNOSTICS_ITEMS,
    },
];

pub(crate) fn render() -> Element {
    let mut selected_topic_id = use_signal(|| GUIDE_TOPICS[0].id);
    let active_topic_id = selected_topic_id();
    let active_topic = GUIDE_TOPICS
        .iter()
        .find(|topic| topic.id == active_topic_id)
        .unwrap_or(&GUIDE_TOPICS[0]);

    rsx! {
        section { class: "guide-panel", id: "guide-overview",
            div { class: "guide-header",
                h1 { "Guide" }
                p { "Help with settings and controls." }
            }

            div { class: "settings-card-grid guide-grid",
                section { class: "sound-card guide-intro",
                    div { class: "sound-card-title",
                        div { class: "startup-copy",
                            h2 { "Using Settings" }
                        }
                    }
                    div { class: "guide-summary-grid",
                        div { class: "guide-summary-item",
                            strong { "Saved automatically" }
                            span { "No Apply button is needed. Invalid values keep the previous setting." }
                        }
                        div { class: "guide-summary-item",
                            strong { "Runs in the background" }
                            span { "Closing Settings keeps MuteGuard running. Use Exit in the tray menu to quit." }
                        }
                        div { class: "guide-summary-item",
                            strong { "Remembers devices" }
                            span { "Microphone and display selections are kept when devices disconnect." }
                        }
                    }
                }

                section { class: "sound-card guide-browser",
                    div { class: "guide-browser-heading",
                        h2 { "Browse sections" }
                        p { "Choose a section." }
                    }
                    div {
                        class: "guide-topic-nav",
                        role: "tablist",
                        aria_label: "Guide sections",
                        for topic in GUIDE_TOPICS {
                            button {
                                key: "{topic.id}",
                                r#type: "button",
                                role: "tab",
                                class: if active_topic_id == topic.id { "guide-topic-button active" } else { "guide-topic-button" },
                                aria_selected: active_topic_id == topic.id,
                                onclick: move |_| selected_topic_id.set(topic.id),
                                span { class: "guide-topic-button-icon",
                                    span { class: "solar-icon {topic.icon}" }
                                }
                                span { "{topic.title}" }
                            }
                        }
                    }
                }

                {guide_topic_card(active_topic)}
            }
        }
    }
}

fn guide_topic_card(topic: &'static GuideTopic) -> Element {
    rsx! {
        section {
            class: "sound-card guide-card",
            id: "{topic.id}",
            role: "tabpanel",
            aria_label: "{topic.title}",
            div { class: "guide-card-heading",
                span { class: "guide-card-icon",
                    span { class: "solar-icon {topic.icon}" }
                }
                div { class: "guide-card-copy",
                    h2 { "{topic.title}" }
                    p { "{topic.description}" }
                }
            }
            dl { class: "guide-items",
                for item in topic.items {
                    div { class: "guide-item",
                        dt { "{item.label}" }
                        dd { "{item.description}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn guide_covers_every_primary_settings_section() {
        for &tab in SettingsTab::PRIMARY {
            assert!(GUIDE_TOPICS.iter().any(|topic| topic.tab == tab));
        }
    }

    #[test]
    fn guide_topics_have_unique_ids_and_complete_items() {
        let mut ids = HashSet::new();
        for topic in GUIDE_TOPICS {
            assert!(ids.insert(topic.id));
            assert!(!topic.items.is_empty());
            assert!(
                topic
                    .items
                    .iter()
                    .all(|item| !item.label.is_empty() && !item.description.is_empty())
            );
        }
    }
}
