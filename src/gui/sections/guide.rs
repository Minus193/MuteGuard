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
        description: "Starts the lightweight background process silently after you sign in. MuteGuard checks the current Run entry once at startup and repairs it only when needed.",
    },
    GuideItem {
        label: "Mute microphone on startup",
        description: "Applies mute once after the default communications microphone becomes available. It does not keep forcing mute afterward.",
    },
    GuideItem {
        label: "Microphone change notifications",
        description: "Shows a Windows notification when the default microphone changes, disconnects, or reconnects. MuteGuard keeps retrying the endpoint connection in the background.",
    },
    GuideItem {
        label: "Accent source",
        description: "Windows color follows the current system accent. Custom color keeps an independent MuteGuard accent for controls, focus rings, and highlights.",
    },
    GuideItem {
        label: "Accent color",
        description: "Appears for the Custom source. Choose a preset or enter and tune an exact color; the change is applied throughout Settings.",
    },
];

const HOTKEY_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Add hotkey",
        description: "Creates a new shortcut and immediately starts recording it. A hotkey is saved only after a complete keyboard or mouse chord is captured.",
    },
    GuideItem {
        label: "Record / Cancel",
        description: "Record replaces the selected chord. Press and release any key combination or supported mouse chord; Cancel keeps the previous shortcut.",
    },
    GuideItem {
        label: "Target",
        description: "Select the default communications microphone, one specific active Windows capture device, or All microphones.",
    },
    GuideItem {
        label: "Specific devices",
        description: "The Windows endpoint ID is saved, not only its display name. A disconnected selection remains listed as unavailable and works again after that device reconnects.",
    },
    GuideItem {
        label: "All microphones",
        description: "Uses the default microphone to decide the next mute state, then applies that state to every active capture endpoint.",
    },
    GuideItem {
        label: "Ignore modifiers",
        description: "Allows the shortcut to fire even when extra Ctrl, Alt, Shift, or Windows keys are held. Keys that are part of the recorded chord are still required.",
    },
    GuideItem {
        label: "Delete",
        description: "Removes only that hotkey. Other shortcuts and their targets are not changed.",
    },
];

const OVERLAY_BEHAVIOR_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Overlay switch",
        description: "Enables or disables every overlay instance. The indicator is click-through and does not take focus or block mouse input.",
    },
    GuideItem {
        label: "Visibility",
        description: "Always visible ignores mute state. Visible when muted and Visible when unmuted follow Core Audio. Temporarily after a change appears only after a mute transition.",
    },
    GuideItem {
        label: "Temporary duration",
        description: "Controls how long the After a change overlay remains visible, from 0.5 to 10 seconds.",
    },
    GuideItem {
        label: "Monitor",
        description: "Select one or more displays. MuteGuard creates one synchronized overlay per available display and remembers disconnected displays for a later reconnect.",
    },
    GuideItem {
        label: "Position",
        description: "Chooses one of nine anchors inside each monitor's usable work area. A safety inset keeps the overlay away from taskbars and screen edges.",
    },
    GuideItem {
        label: "Preview",
        description: "Temporarily forces the overlay to appear while this checkbox is enabled. It is not saved and turns off when you leave Overlay or close Settings.",
    },
];

const OVERLAY_CONTENT_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Style",
        description: "Choose Microphone icon, Icon and text, Text only, or Status dot. Controls that do not apply to the selected style are hidden.",
    },
    GuideItem {
        label: "Icon",
        description: "Selects the microphone artwork used by the real overlay. Its muted or unmuted form follows the current visibility context and microphone state.",
    },
    GuideItem {
        label: "Icon color",
        description: "Colored uses your chosen color, Monochrome uses a neutral foreground, and System color follows the Windows accent.",
    },
    GuideItem {
        label: "Color",
        description: "Appears for Colored icons and accepts presets, visual tuning, RGB feedback, or an exact HEX value.",
    },
    GuideItem {
        label: "Muted / Unmuted label",
        description: "Sets the text shown for each microphone state when the selected content style includes text.",
    },
    GuideItem {
        label: "Font",
        description: "Uses an installed Windows font family for overlay text. The selected family remains saved even if it is temporarily unavailable.",
    },
    GuideItem {
        label: "Font weight",
        description: "Adjusts text thickness from 100 to 900 without changing the selected font family.",
    },
    GuideItem {
        label: "Scale",
        description: "Changes the complete overlay geometry from 10% to 400%, including icon, text, spacing, and background.",
    },
    GuideItem {
        label: "Content opacity",
        description: "Changes only the icon, text, or dot opacity. Background opacity is controlled separately.",
    },
];

const OVERLAY_BACKGROUND_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Background style",
        description: "Dark and Light provide a contrasting surface. Transparent removes the fill while leaving the selected content visible.",
    },
    GuideItem {
        label: "Background opacity",
        description: "Controls only the background fill from fully transparent to fully opaque.",
    },
    GuideItem {
        label: "Corner radius",
        description: "Adjusts the background corner roundness from square corners to a 24 px radius.",
    },
    GuideItem {
        label: "Show border",
        description: "Draws an antialiased outline around the overlay background.",
    },
    GuideItem {
        label: "Border color",
        description: "Appears when Show border is enabled and uses the same exact-color editor as the other custom colors.",
    },
];

const TRAY_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Tray icon style",
        description: "Logo always shows the MuteGuard logo. Mic status shows the chosen microphone artwork. Color dot uses a compact state indicator.",
    },
    GuideItem {
        label: "Microphone icons",
        description: "Selects the artwork for Mic status. More expands the full icon library; Less returns to the featured set.",
    },
    GuideItem {
        label: "Icon color",
        description: "Colored uses your custom color, Monochrome uses a neutral tone, and System color follows the Windows accent.",
    },
    GuideItem {
        label: "Color",
        description: "Appears for Colored status icons and controls the tray microphone color without changing the overlay color.",
    },
    GuideItem {
        label: "Tray menu",
        description: "Left-click opens or focuses Settings. Right-click offers Mute or Unmute, Settings, and Exit; the mute command is disabled while no microphone is available.",
    },
];

const SOUND_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Sound feedback",
        description: "Enables a short confirmation after real mute and unmute state changes. Disabling it does not affect the preview buttons.",
    },
    GuideItem {
        label: "Volume",
        description: "Adjusts only MuteGuard feedback tones from 0% to 100%. It never changes the Windows microphone level.",
    },
    GuideItem {
        label: "Mute / Unmute source",
        description: "Each state can independently use the built-in tone or its own custom WAV file.",
    },
    GuideItem {
        label: "Choose WAV",
        description: "Imports uncompressed 16-bit PCM audio up to 5 seconds and 12 MB. Choosing another file replaces the previous custom sound for that state.",
    },
    GuideItem {
        label: "Missing custom files",
        description: "If Custom is selected but no valid file is available, MuteGuard safely falls back to the built-in tone.",
    },
    GuideItem {
        label: "Preview mute / unmute",
        description: "Plays the configured sound without changing microphone state. Rapid requests can overlap, so a new cue does not cut off or wait for the previous one.",
    },
];

const DIAGNOSTICS_ITEMS: &[GuideItem] = &[
    GuideItem {
        label: "Refresh",
        description: "Re-reads the current application, Windows, Core Audio, input, and overlay status. Diagnostics is read-only and changes no settings.",
    },
    GuideItem {
        label: "Application",
        description: "Shows version, architecture, background-process, configuration, and Windows-startup status.",
    },
    GuideItem {
        label: "Windows",
        description: "Shows the Windows build and availability of required local runtimes and APIs.",
    },
    GuideItem {
        label: "Audio",
        description: "Shows Core Audio availability, current microphone state, endpoint identity, notification monitoring, and sound-file status.",
    },
    GuideItem {
        label: "Input and overlay",
        description: "Summarizes configured hotkeys, overlay state, and configured versus currently detected displays.",
    },
    GuideItem {
        label: "Copy diagnostics",
        description: "Copies a support report containing the visible status values without credentials or complete personal file paths.",
    },
];

const GUIDE_TOPICS: &[GuideTopic] = &[
    GuideTopic {
        tab: SettingsTab::General,
        id: "guide-general",
        title: "General",
        description: "Startup, device notifications, and the Settings accent.",
        icon: "icon-settings",
        items: GENERAL_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Hotkeys,
        id: "guide-hotkeys",
        title: "Hotkeys",
        description: "Shortcut recording, microphone targets, and matching rules.",
        icon: "icon-keyboard",
        items: HOTKEY_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Overlay,
        id: "guide-overlay-behavior",
        title: "Overlay — behavior",
        description: "When, where, and on which displays the overlay appears.",
        icon: "icon-monitor",
        items: OVERLAY_BEHAVIOR_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Overlay,
        id: "guide-overlay-content",
        title: "Overlay — content",
        description: "Artwork, text, scale, color, and content opacity.",
        icon: "icon-mic",
        items: OVERLAY_CONTENT_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Overlay,
        id: "guide-overlay-background",
        title: "Overlay — background",
        description: "Surface style, opacity, corners, and border.",
        icon: "icon-contrast",
        items: OVERLAY_BACKGROUND_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Tray,
        id: "guide-tray",
        title: "Tray",
        description: "Notification-area appearance and mouse actions.",
        icon: "icon-widget",
        items: TRAY_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Sound,
        id: "guide-sound",
        title: "Sound",
        description: "Feedback level, sources, custom WAV files, and previews.",
        icon: "icon-speaker",
        items: SOUND_ITEMS,
    },
    GuideTopic {
        tab: SettingsTab::Diagnostics,
        id: "guide-diagnostics",
        title: "Diagnostics",
        description: "A private, read-only view of MuteGuard's local status.",
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
                p { "A complete reference for every MuteGuard section and setting." }
            }

            div { class: "settings-card-grid guide-grid",
                section { class: "sound-card guide-intro",
                    div { class: "sound-card-title",
                        div { class: "startup-copy",
                            h2 { "How Settings works" }
                            p { "MuteGuard applies configuration changes as soon as they are accepted." }
                        }
                    }
                    div { class: "guide-summary-grid",
                        div { class: "guide-summary-item",
                            strong { "Saved immediately" }
                            span { "There is no separate Apply button. Invalid input is rejected without replacing the last valid value." }
                        }
                        div { class: "guide-summary-item",
                            strong { "Runs in the background" }
                            span { "Closing Settings leaves the lightweight tray process running. Use Exit from the tray menu to stop MuteGuard." }
                        }
                        div { class: "guide-summary-item",
                            strong { "Reconnect friendly" }
                            span { "Saved device and display choices remain available when hardware is temporarily disconnected." }
                        }
                    }
                }

                section { class: "sound-card guide-browser",
                    div { class: "guide-browser-heading",
                        h2 { "Browse sections" }
                        p { "Choose a section to see its settings and behavior." }
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
