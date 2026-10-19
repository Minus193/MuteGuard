use dioxus::prelude::*;
use windows::Win32::{
    Foundation::{FreeLibrary, HMODULE},
    System::LibraryLoader::{
        GetProcAddress, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
        LoadLibraryExW,
    },
};
use windows::core::{PCSTR, PCWSTR};

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
struct Snapshot {
    size: u32,
    status: i32,
    error: i32,
    rate: u32,
    channels: u32,
    sequence: u32,
    revision: u32,
    noise_active: u32,
    input_peak: f32,
    output_peak: f32,
    input: [f32; 64],
    output: [f32; 64],
    monitor_status: i32,
    monitor_error: i32,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            status: 0,
            error: 0,
            rate: 0,
            channels: 0,
            sequence: 0,
            revision: 0,
            noise_active: 0,
            input_peak: -100.0,
            output_peak: -100.0,
            input: [-100.0; 64],
            output: [-100.0; 64],
            monitor_status: 0,
            monitor_error: 0,
        }
    }
}

type Start = unsafe extern "C" fn(*const u16) -> *mut std::ffi::c_void;
type Read = unsafe extern "C" fn(*mut std::ffi::c_void, *mut Snapshot, u32) -> i32;
type Update = unsafe extern "C" fn(*mut std::ffi::c_void);
type Stop = unsafe extern "C" fn(*mut std::ffi::c_void);
type Listen = unsafe extern "C" fn(*mut std::ffi::c_void, i32);

struct Preview {
    library: HMODULE,
    handle: *mut std::ffi::c_void,
    read: Read,
    update: Update,
    stop: Stop,
    listen: Listen,
}

impl Preview {
    fn start(device: &str) -> anyhow::Result<Self> {
        let path = crate::apo_test_directory()?.join("x64/Release/MuteGuardAPO.dll");
        let path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        unsafe {
            let library = LoadLibraryExW(
                PCWSTR(path.as_ptr()),
                None,
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            )?;
            let symbols = [
                b"MuteGuardPreviewStart\0".as_slice(),
                b"MuteGuardPreviewRead\0",
                b"MuteGuardPreviewUpdate\0",
                b"MuteGuardPreviewStop\0",
                b"MuteGuardPreviewListen\0",
            ]
            .map(|name| GetProcAddress(library, PCSTR(name.as_ptr())));
            let [
                Some(start),
                Some(read),
                Some(update),
                Some(stop),
                Some(listen),
            ] = symbols
            else {
                let _ = FreeLibrary(library);
                anyhow::bail!(
                    "This audio component does not include the analyzer. Use the new MuteGuard build."
                );
            };
            let start: Start = std::mem::transmute(start);
            let device = device.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            let handle = start(device.as_ptr());
            if handle.is_null() {
                let _ = FreeLibrary(library);
                anyhow::bail!("Could not start the microphone preview.");
            }
            Ok(Self {
                library,
                handle,
                read: std::mem::transmute::<unsafe extern "system" fn() -> isize, Read>(read),
                update: std::mem::transmute::<unsafe extern "system" fn() -> isize, Update>(update),
                stop: std::mem::transmute::<unsafe extern "system" fn() -> isize, Stop>(stop),
                listen: std::mem::transmute::<unsafe extern "system" fn() -> isize, Listen>(listen),
            })
        }
    }
    fn snapshot(&self) -> anyhow::Result<Snapshot> {
        let mut snapshot = Snapshot::default();
        anyhow::ensure!(
            unsafe {
                (self.read)(
                    self.handle,
                    &mut snapshot,
                    std::mem::size_of::<Snapshot>() as u32,
                )
            } == 1,
            "Analyzer and audio component versions do not match."
        );
        anyhow::ensure!(
            snapshot.status >= 0,
            "Raw microphone capture failed (0x{:08X}). Check microphone access and close apps using exclusive mode. This device may not support raw capture.",
            snapshot.error as u32
        );
        Ok(snapshot)
    }
}

use std::os::windows::ffi::OsStrExt;

impl Drop for Preview {
    fn drop(&mut self) {
        unsafe {
            (self.stop)(self.handle);
            let _ = FreeLibrary(self.library);
        }
    }
}

fn spectrum_path(values: &[f32; 64]) -> String {
    use std::fmt::Write;
    let mut path = String::new();
    for (index, value) in values.iter().enumerate() {
        let x = (index as f32 + 0.5) * 720.0 / 64.0;
        let y = value.clamp(-100.0, 0.0).abs() * 2.0;
        let _ = write!(
            path,
            "{} {x:.1} {y:.1} ",
            if index == 0 { "M" } else { "L" }
        );
    }
    path
}

#[component]
pub(super) fn Analyzer(device_id: String, effects: Signal<crate::ApoEffects>) -> Element {
    let mut preview = use_signal(|| None::<Preview>);
    let mut snapshot = use_signal(Snapshot::default);
    let mut error = use_signal(String::new);
    let mut running = use_signal(|| false);
    let mut volume = use_signal(|| 25_i32);
    let device_for_reset = device_id.clone();
    use_effect(use_reactive(&device_for_reset, move |_| {
        preview.set(None);
        running.set(false);
        snapshot.set(Snapshot::default());
        error.set(String::new());
        volume.set(25);
    }));
    use_effect(move || {
        let _ = effects();
        if let Some(preview) = preview.peek().as_ref() {
            unsafe {
                (preview.update)(preview.handle);
            }
        }
    });
    let mut polling = use_future(move || async move {
        if !running() {
            return;
        }
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(67)).await;
            if !running() {
                return;
            }
            let result = preview.peek().as_ref().map(Preview::snapshot);
            match result {
                Some(Ok(next)) => snapshot.set(next),
                Some(Err(problem)) => {
                    error.set(problem.to_string());
                    preview.set(None);
                    running.set(false);
                }
                None => {}
            }
        }
    });
    // Dropping the signal's value joins the worker and releases the microphone on unmount.
    let data = snapshot();
    let input_path = spectrum_path(&data.input);
    let output_path = spectrum_path(&data.output);
    let live = running() && data.status == 1;
    let listening = live && data.monitor_status == 1;
    let active_count = effects()
        .order
        .iter()
        .filter(|module| super::effects::module_info(**module, effects()).2)
        .count();
    let description = if live {
        let layout = if data.channels == 1 {
            "mono".into()
        } else {
            format!("{} channels", data.channels)
        };
        format!(
            "{} kHz · {} · {} active modules",
            data.rate as f32 / 1000.0,
            layout,
            active_count
        )
    } else if running() {
        "Opening microphone…".into()
    } else {
        "Start a preview and speak to compare your signal.".into()
    };
    rsx! {
        section { class: "sound-card analyzer-card", aria_label: "Live microphone preview",
            div { class: "sound-card-title",
                div { class: "startup-copy",
                    span { class: "effect-eyebrow", "LISTEN WITH YOUR EYES" }
                    h2 { "Live microphone preview" }
                    p { "{description}" }
                }
                button { class: "secondary", r#type: "button", disabled: device_id.is_empty(),
                    onclick: move |_| {
                        if running() { polling.cancel(); preview.set(None); running.set(false); }
                        else {
                            error.set(String::new()); snapshot.set(Snapshot::default());
                            volume.set(25);
                            match Preview::start(&device_id) {
                                Ok(handle) => { preview.set(Some(handle)); running.set(true); polling.restart(); }
                                Err(problem) => error.set(problem.to_string()),
                            }
                        }
                    },
                    if running() { "Stop preview" } else { "Start preview" }
                }
            }
            div { class: "analyzer-legend",
                span { class: "analyzer-before", "● Input · before effects" }
                span { class: "analyzer-after", "● Output · current settings" }
                span { class: "analyzer-state", if live { "LIVE" } else { "PREVIEW" } }
            }
            div { class: "spectrum-frame",
                div { class: "spectrum-axis-unit", "dBFS" }
                div { class: "spectrum-chart",
                    div { class: "spectrum-y-axis", aria_hidden: "true",
                        for db in [0, 20, 40, 60, 80, 100] {
                            span { if db == 0 { "0" } else { "−{db}" } }
                        }
                    }
                    div { class: "spectrum-plot",
                svg { class: "spectrum", view_box: "0 0 720 200", preserve_aspect_ratio: "none", role: "img", "aria-label": "Frequency spectrum, input and processed output. Horizontal axis 20 Hz to 20 kHz, vertical axis minus 100 to zero dBFS.",
                    for db in [0, 20, 40, 60, 80, 100] {
                        line { x1: "0", x2: "720", y1: "{db * 2}", y2: "{db * 2}", class: "spectrum-grid" }
                    }
                    for hz in [20.0_f32, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0] {
                        line { x1: "{(hz / 20.0).log10() * 240.0}", x2: "{(hz / 20.0).log10() * 240.0}", y1: "0", y2: "200", class: "spectrum-grid" }
                    }
                    if live {
                        path { d: "{input_path}", class: "spectrum-input" }
                        path { d: "{output_path} L 714.4 200 L 5.6 200 Z", class: "spectrum-fill" }
                        path { d: "{output_path}", class: "spectrum-output" }
                    }
                }
                if !live { div { class: "spectrum-placeholder", if running() { "Waiting for microphone" } else { "Your microphone spectrum will appear here" } } }
                    }
                }
                div { class: "spectrum-x-axis", aria_hidden: "true",
                    for (hz, label) in [(20.0_f32, "20"), (50.0,"50"), (100.0,"100"), (200.0,"200"), (500.0,"500"), (1000.0,"1k"), (2000.0,"2k"), (5000.0,"5k"), (10000.0,"10k"), (20000.0,"20k")] {
                        span { class: if [50.0, 200.0, 2000.0, 10000.0].contains(&hz) { "spectrum-minor-label" } else { "" }, style: "left: {(hz / 20.0).log10() * 100.0 / 3.0}%", "{label}" }
                    }
                }
                div { class: "spectrum-axis-caption", "Frequency (Hz) · logarithmic" }
            }
            div { class: "analyzer-meters",
                LevelMeter { label: "Input peak", value: data.input_peak, live }
                LevelMeter { label: "Output peak", value: data.output_peak, live }
            }
            div { class: "analyzer-listen",
                button { r#type: "button", class: "secondary", disabled: !live,
                    aria_pressed: listening,
                    title: if live { "Play processed microphone audio through the default Windows output" } else { "Start the preview to listen" },
                    onclick: move |_| {
                        if let Some(preview) = preview.peek().as_ref() {
                            unsafe { (preview.listen)(preview.handle, if listening { -1 } else { volume() }); }
                        }
                    },
                    if listening { "Stop listening" } else { "Listen" }
                }
                if live {
                    label { class: "analyzer-volume",
                        span { "Listening volume · {volume}%" }
                        input { r#type: "range", min: "0", max: "100", value: "{volume}",
                            oninput: move |event| {
                                if let Ok(percent) = event.value().parse::<i32>() {
                                    let percent = percent.clamp(0, 100);
                                    volume.set(percent);
                                    if listening && let Some(preview) = preview.peek().as_ref() {
                                        unsafe { (preview.listen)(preview.handle, percent); }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "analyzer-note", "Use headphones to avoid feedback. Listening uses your default Windows output and adds a short delay." }
            if live && data.monitor_status < 0 {
                p { role: "alert", class: "analyzer-error", "Could not play the preview (0x{data.monitor_error as u32:08X}). Check your Windows output device and try Listen again." }
            }
            if !error().is_empty() { p { role: "alert", class: "analyzer-error", "{error}" } }
            if live && effects().noise && effects().noise_strength > 0 && data.noise_active == 0 {
                p { role: "status", "RNNoise is bypassed: it needs 48 kHz and an installed noise model." }
            }
            details { class: "effect-help",
                summary { "How to read this" }
                p { "Left: bass and rumble. Middle: the body of your voice. Right: clarity, S sounds and hiss. A lower output trace means less energy at that frequency. The spectrum shows sound, not an EQ response curve." }
                p { "Try silence for noise reduction, a steady vowel for EQ, and a loud sentence for compression. Peak meters show short-term level in dBFS; 0 dBFS is the digital ceiling. Spectrum: channel 1, Hann-window FFT; bass detail is limited by the analysis window." }
                p { "The preview uses raw microphone capture and the MuteGuard engine locally. Raw capture can retain always-on device processing. RNNoise adds about 20 ms of delay, so fast sounds may appear at slightly different times in the two traces." }
            }
            p { class: "analyzer-note", "Edits update this preview immediately. Reopen other recording apps to apply there. Nothing is saved; leaving this tab stops listening." }
        }
    }
}

#[component]
fn LevelMeter(label: &'static str, value: f32, live: bool) -> Element {
    let width = if live {
        value.clamp(-100.0, 0.0) + 100.0
    } else {
        0.0
    };
    let reading = if live {
        format!("{value:.1} dBFS")
    } else {
        "—".into()
    };
    rsx! {
        div { class: "analyzer-meter",
            div { span { "{label}" } strong { "{reading}" } }
            div { class: "meter-track", role: "meter", aria_label: label, aria_valuemin: "-100", aria_valuemax: "0", aria_valuenow: "{value.clamp(-100.0, 0.0)}",
                span { class: if value >= 0.0 && live { "meter-fill is-clipping" } else { "meter-fill" }, style: "width: {width}%" }
            }
            if live && value >= 0.0 {
                small { class: "analyzer-error",
                    if label == "Input peak" { "Input clipping: lower the microphone input level." }
                    else { "Output clipping: reduce EQ boosts or enable the limiter." }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spectrum_coordinates_and_abi_are_stable() {
        assert_eq!(std::mem::size_of::<Snapshot>(), 560);
        let path = spectrum_path(&[-100.0; 64]);
        assert!(path.starts_with("M 5.6 200.0"));
        assert!(path.contains("714.4 200.0"));
        assert!(!spectrum_path(&[12.0; 64]).contains("-"));
    }
}
