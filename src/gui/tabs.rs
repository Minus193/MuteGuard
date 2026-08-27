use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Hotkeys,
    Overlay,
    Tray,
    Sound,
    Diagnostics,
    Guide,
}

impl SettingsTab {
    pub(crate) const PRIMARY: &'static [Self] = &[
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
            Self::Guide => "Guide",
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
            Self::Guide => "icon-help",
        }
    }
}

pub(crate) fn render(active_tab: Signal<SettingsTab>) -> Element {
    rsx! {
        nav { class: "sidebar", aria_label: "MuteGuard settings",
            div { class: "sidebar-scroll",
                div { class: "sidebar-primary",
                    for &tab in SettingsTab::PRIMARY {
                        SidebarItem { tab, active_tab }
                    }
                }
                div { class: "sidebar-footer",
                    SidebarItem {
                        tab: SettingsTab::Guide,
                        active_tab,
                    }
                }
            }
        }
    }
}

#[component]
fn SidebarItem(tab: SettingsTab, mut active_tab: Signal<SettingsTab>) -> Element {
    rsx! {
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
