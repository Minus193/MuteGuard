use dioxus::prelude::*;

use super::{SettingsSnapshot, tabs::SettingsTab};

mod diagnostics;
mod general;
mod hotkeys;
mod overlay;
mod sound;
mod tray_icon;

pub(crate) fn render(tab: SettingsTab, settings: Signal<SettingsSnapshot>) -> Element {
    match tab {
        SettingsTab::General => rsx! { GeneralSection { settings } },
        SettingsTab::Hotkeys => rsx! { HotkeysSection { settings } },
        SettingsTab::Overlay => rsx! { OverlaySection { settings } },
        SettingsTab::Tray => rsx! { TraySection { settings } },
        SettingsTab::Sound => rsx! { SoundSection { settings } },
        SettingsTab::Diagnostics => rsx! { DiagnosticsSection {} },
    }
}

#[component]
fn GeneralSection(settings: Signal<SettingsSnapshot>) -> Element {
    general::render(settings)
}

#[component]
fn HotkeysSection(settings: Signal<SettingsSnapshot>) -> Element {
    hotkeys::render(settings)
}

#[component]
fn OverlaySection(settings: Signal<SettingsSnapshot>) -> Element {
    overlay::render(settings)
}

#[component]
fn TraySection(settings: Signal<SettingsSnapshot>) -> Element {
    tray_icon::render(settings)
}

#[component]
fn SoundSection(settings: Signal<SettingsSnapshot>) -> Element {
    sound::render(settings)
}

#[component]
fn DiagnosticsSection() -> Element {
    diagnostics::render()
}
