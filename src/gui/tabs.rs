use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Hotkeys,
    Overlay,
    Tray,
    Sound,
    Diagnostics,
}

impl SettingsTab {
    pub(crate) const ALL: &'static [Self] = &[
        Self::General,
        Self::Hotkeys,
        Self::Overlay,
        Self::Tray,
        Self::Sound,
        Self::Diagnostics,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Hotkeys => "Hotkeys",
            Self::Overlay => "Overlay",
            Self::Tray => "Tray",
            Self::Sound => "Sound",
            Self::Diagnostics => "Diagnostics",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::General => "icon-settings",
            Self::Hotkeys => "icon-keyboard",
            Self::Overlay => "icon-monitor",
            Self::Tray => "icon-widget",
            Self::Sound => "icon-speaker",
            Self::Diagnostics => "icon-diagnostics",
        }
    }
}

pub(crate) fn render(mut active_tab: Signal<SettingsTab>) -> Element {
    rsx! {
        nav { class: "sidebar", aria_label: "MuteGuard settings",
            div { class: "sidebar-scroll",
                for &tab in SettingsTab::ALL {
                    button {
                        class: if active_tab() == tab { "nav-item active" } else { "nav-item" },
                        aria_pressed: active_tab() == tab,
                        onclick: move |_| {
                            active_tab.set(tab);
                            reset_content_scroll();
                        },
                        span { class: "nav-icon-stack",
                            span { class: "solar-icon nav-icon nav-icon-line {tab.icon()}" }
                            span { class: "solar-icon nav-icon nav-icon-filled {tab.icon()}" }
                        }
                        span { class: "nav-label", "{tab.label()}" }
                    }
                }
            }
        }
    }
}

fn reset_content_scroll() {
    spawn(async move {
        let _ = dioxus::document::eval(
            r#"
const scroller = document.querySelector('.content-scroll');
if (scroller) scroller.scrollTop = 0;
"#,
        )
        .await;
    });
}
