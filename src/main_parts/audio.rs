fn current_mute_state() -> Result<bool> {
    mic_mute_state()
}

fn mic_mute_state() -> Result<bool> {
    let volume = capture_volume()?;
    Ok(unsafe { volume.GetMute()? }.as_bool())
}

fn set_mute_to_inverse(target: Option<&str>) -> Result<()> {
    if is_all_microphones_target(target) {
        // The default endpoint defines the direction of a grouped toggle.
        // Enumeration happens only here, in direct response to a user action.
        let next = !current_mute_state()?;
        return set_all_capture_devices_mute(next);
    }

    let volume = capture_volume_for_target(target)?;
    let next = !unsafe { volume.GetMute()? }.as_bool();
    unsafe {
        volume
            .SetMute(next, null())
            .context("toggle microphone mute")?;
    }
    Ok(())
}

fn set_mute(target: Option<&str>, muted: bool) -> Result<()> {
    if is_all_microphones_target(target) {
        return set_all_capture_devices_mute(muted);
    }

    let volume = capture_volume_for_target(target)?;
    if !mute_change_required(unsafe { volume.GetMute()? }.as_bool(), muted) {
        return Ok(());
    }
    unsafe {
        volume
            .SetMute(muted, null())
            .context("set microphone mute")?;
    }
    Ok(())
}

fn mute_change_required(current: bool, requested: bool) -> bool {
    current != requested
}

fn is_all_microphones_target(target: Option<&str>) -> bool {
    matches!(target, Some(id) if id == HOTKEY_TARGET_ALL_MICROPHONES)
}

fn audio_device_enumerator() -> Result<IMMDeviceEnumerator> {
    unsafe {
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .context("create audio device enumerator")
    }
}

fn default_capture_devices() -> Result<DefaultCaptureDevices> {
    let enumerator = audio_device_enumerator()?;
    Ok(DefaultCaptureDevices {
        communications: unsafe { default_capture_device_id(&enumerator, eCommunications) },
        console: unsafe { default_capture_device_id(&enumerator, eConsole) },
        multimedia: unsafe { default_capture_device_id(&enumerator, eMultimedia) },
    })
}

pub(crate) fn active_capture_devices() -> Vec<CaptureDeviceOption> {
    enumerate_active_capture_devices().unwrap_or_default()
}

const MUTEGUARD_APO_CLSID: &str = "{A4EC941D-F1BA-4C19-B195-4092858D9D90}";
const EQUALIZER_APO_CLSID: &str = "{EACD2258-FCAC-4FF4-B36D-419E924A6D79}";

#[derive(Clone, Copy)]
pub(crate) struct ApoEffects {
    pub noise: bool,
    pub gate: bool,
    pub compressor: bool,
    pub noise_strength: i32,
    pub gate_threshold_db: i32,
    pub gate_hold_ms: i32,
    pub gate_release_ms: i32,
    pub compressor_threshold_db: i32,
    pub compressor_ratio_tenths: i32,
    pub compressor_attack_ms: i32,
    pub compressor_release_ms: i32,
}

impl Default for ApoEffects {
    fn default() -> Self {
        Self {
            noise: false,
            gate: false,
            compressor: false,
            noise_strength: 100,
            gate_threshold_db: -45,
            gate_hold_ms: 100,
            gate_release_ms: 150,
            compressor_threshold_db: -18,
            compressor_ratio_tenths: 25,
            compressor_attack_ms: 10,
            compressor_release_ms: 150,
        }
    }
}

impl ApoEffects {
    fn clamped(mut self) -> Self {
        self.noise_strength = self.noise_strength.clamp(0, 100);
        self.gate_threshold_db = self.gate_threshold_db.clamp(-80, -10);
        self.gate_hold_ms = self.gate_hold_ms.clamp(0, 500);
        self.gate_release_ms = self.gate_release_ms.clamp(10, 1000);
        self.compressor_threshold_db = self.compressor_threshold_db.clamp(-40, 0);
        self.compressor_ratio_tenths = self.compressor_ratio_tenths.clamp(10, 100);
        self.compressor_attack_ms = self.compressor_attack_ms.clamp(1, 100);
        self.compressor_release_ms = self.compressor_release_ms.clamp(20, 1000);
        self
    }
}

fn apo_effects_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(std::path::PathBuf::from(
        std::env::var_os("ProgramData").context("ProgramData is unavailable")?,
    )
    .join("MuteGuard/APO-test/effects.ini"))
}

pub(crate) fn read_apo_effects() -> anyhow::Result<ApoEffects> {
    let content = match std::fs::read_to_string(apo_effects_path()?) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(ApoEffects::default()),
        Err(error) => return Err(error.into()),
    };
    Ok(parse_apo_effects(&content))
}

fn parse_apo_effects(content: &str) -> ApoEffects {
    let value = |key: &str, fallback: i32| {
        content.lines().find_map(|line| {
            let (name, raw) = line.trim().split_once('=')?;
            (name.trim() == key).then(|| raw.trim().parse::<i32>().ok()).flatten()
        }).unwrap_or(fallback)
    };
    ApoEffects {
        noise: value("Noise", 0) == 1,
        gate: value("Gate", 0) == 1,
        compressor: value("Compressor", 0) == 1,
        noise_strength: value("NoiseStrength", 100),
        gate_threshold_db: value("GateThresholdDb", -45),
        gate_hold_ms: value("GateHoldMs", 100),
        gate_release_ms: value("GateReleaseMs", 150),
        compressor_threshold_db: value("CompressorThresholdDb", -18),
        compressor_ratio_tenths: value("CompressorRatioTenths", 25),
        compressor_attack_ms: value("CompressorAttackMs", 10),
        compressor_release_ms: value("CompressorReleaseMs", 150),
    }.clamped()
}

pub(crate) fn write_apo_effects(effects: ApoEffects) -> anyhow::Result<()> {
    let path = apo_effects_path()?;
    anyhow::ensure!(path.parent().is_some_and(std::path::Path::exists), "APO is not installed");
    let effects = effects.clamped();
    std::fs::write(path, format!(
        "[Effects]\nNoise={}\nGate={}\nCompressor={}\nNoiseStrength={}\nGateThresholdDb={}\nGateHoldMs={}\nGateReleaseMs={}\nCompressorThresholdDb={}\nCompressorRatioTenths={}\nCompressorAttackMs={}\nCompressorReleaseMs={}\n",
        u8::from(effects.noise), u8::from(effects.gate), u8::from(effects.compressor),
        effects.noise_strength, effects.gate_threshold_db, effects.gate_hold_ms,
        effects.gate_release_ms, effects.compressor_threshold_db, effects.compressor_ratio_tenths,
        effects.compressor_attack_ms, effects.compressor_release_ms
    ))?;
    Ok(())
}

fn apo_test_directory() -> anyhow::Result<std::path::PathBuf> {
    let packaged = std::env::current_exe()?
        .parent()
        .context("MuteGuard executable directory is unavailable")?
        .join("apo");
    if packaged.join("live-test.ps1").exists() {
        Ok(packaged)
    } else {
        Ok(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("apo"))
    }
}

pub(crate) fn apo_test_assets_available() -> bool {
    apo_test_directory().is_ok_and(|directory| {
        directory.join("live-test.ps1").is_file()
            && directory.join("x64/Release/MuteGuardAPO.dll").is_file()
            && directory.join("rnnoise/weights_blob.bin").is_file()
    })
}

pub(crate) fn apo_test_has_saved_state() -> bool {
    apo_effects_path().is_ok_and(|path| {
        path.with_file_name("live-test-state.json").is_file()
    })
}

pub(crate) fn run_apo_test_action(install: bool, device_id: &str) -> anyhow::Result<()> {
    use windows::Win32::{
        Foundation::WAIT_OBJECT_0,
        System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
        UI::{Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW}, WindowsAndMessaging::SW_HIDE},
    };

    let directory = apo_test_directory()?;
    let script = directory.join("live-test.ps1");
    anyhow::ensure!(script.is_file(), "APO test script is unavailable");
    if install {
        anyhow::ensure!(directory.join("x64/Release/MuteGuardAPO.dll").is_file(), "APO DLL is unavailable");
        anyhow::ensure!(directory.join("rnnoise/weights_blob.bin").is_file(), "RNNoise model is unavailable");
    }
    let action = if install { "Install" } else { "Restore" };
    let endpoint_argument = if install {
        let guid = capture_endpoint_guid(device_id).context("microphone endpoint ID is unavailable")?;
        format!(" -EndpointGuid \"{guid}\"")
    } else {
        String::new()
    };
    let arguments = wide(&format!(
        "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File \"{}\" -Action {action}{endpoint_argument}",
        script.display()
    ));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: windows::core::w!("runas"),
        lpFile: windows::core::w!("powershell.exe"),
        lpParameters: PCWSTR(arguments.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info).context("request Windows administrator approval")? };
    anyhow::ensure!(!info.hProcess.is_invalid(), "Windows did not start the APO action");
    let result = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    let mut exit_code = 1;
    let exit_result = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess)? };
    anyhow::ensure!(result == WAIT_OBJECT_0, "Windows could not wait for the APO action");
    exit_result.context("read APO action result")?;
    if exit_code != 0 {
        let detail = std::fs::read_to_string(directory.join("last-action-error.txt"))
            .unwrap_or_else(|_| format!("exit code {exit_code}"));
        anyhow::bail!("{}", detail.trim());
    }
    Ok(())
}

pub(crate) fn capture_effect_status(device_id: &str) -> String {
    let Some(guid) = capture_endpoint_guid(device_id) else {
        return "Endpoint ID unavailable".to_string();
    };
    let key = wide(&format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture\{guid}\FxProperties"
    ));
    let value_name = wide("{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5");
    let mut value = [0_u16; 64];
    let mut bytes = (value.len() * size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return "Stream effect unavailable".to_string();
    }
    let clsid = wide_buf_to_string(&value);
    if clsid.eq_ignore_ascii_case(MUTEGUARD_APO_CLSID) {
        "MuteGuard prototype linked".to_string()
    } else if clsid.eq_ignore_ascii_case(EQUALIZER_APO_CLSID) {
        "Equalizer APO linked".to_string()
    } else {
        format!("Other stream effect: {clsid}")
    }
}

fn capture_endpoint_guid(device_id: &str) -> Option<&str> {
    let start = device_id.rfind(".{")? + 1;
    let guid = device_id.get(start..)?;
    (guid.len() == 38
        && guid.starts_with('{')
        && guid.ends_with('}')
        && guid[1..37].bytes().all(|byte| byte.is_ascii_hexdigit() || byte == b'-'))
    .then_some(guid)
}

#[cfg(test)]
mod effect_status_tests {
    use super::{capture_endpoint_guid, parse_apo_effects};

    #[test]
    fn extracts_capture_endpoint_guid() {
        assert_eq!(
            capture_endpoint_guid("{0.0.1.00000000}.{ae68b532-cdb6-4253-8b86-0c6137188f7d}"),
            Some("{ae68b532-cdb6-4253-8b86-0c6137188f7d}")
        );
        assert_eq!(capture_endpoint_guid("not an endpoint"), None);
    }

    #[test]
    fn old_effect_file_keeps_the_original_tuning() {
        let settings = parse_apo_effects("[Effects]\nNoise=1\nGate=1\nCompressor=1\n");
        assert!(settings.noise && settings.gate && settings.compressor);
        assert_eq!(settings.noise_strength, 100);
        assert_eq!(settings.gate_threshold_db, -45);
        assert_eq!(settings.compressor_ratio_tenths, 25);
    }

    #[test]
    fn effect_values_are_bounded_before_saving() {
        let settings = parse_apo_effects("NoiseStrength=400\nGateThresholdDb=-200\nCompressorRatioTenths=0\n");
        assert_eq!(settings.noise_strength, 100);
        assert_eq!(settings.gate_threshold_db, -80);
        assert_eq!(settings.compressor_ratio_tenths, 10);
    }
}

pub(crate) fn capture_device_name(device_id: &str) -> Option<String> {
    let enumerator = audio_device_enumerator().ok()?;
    let device_id = wide(device_id);
    let device = unsafe { enumerator.GetDevice(PCWSTR(device_id.as_ptr())) }.ok()?;
    unsafe { endpoint_friendly_name(&device) }.ok()
}

fn enumerate_active_capture_devices() -> Result<Vec<CaptureDeviceOption>> {
    let enumerator = audio_device_enumerator()?;
    let collection = unsafe {
        enumerator
            .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
            .context("enumerate active capture endpoints")?
    };
    let count = unsafe { collection.GetCount().context("count active capture endpoints")? };
    let mut devices = Vec::with_capacity(count as usize);
    for index in 0..count {
        let Ok(device) = (unsafe { collection.Item(index) }) else {
            continue;
        };
        let Ok(id) = (unsafe { endpoint_device_id(&device) }) else {
            continue;
        };
        let name = unsafe { endpoint_friendly_name(&device) }
            .unwrap_or_else(|_| "Microphone".to_string());
        devices.push(CaptureDeviceOption { id, name });
    }
    devices.sort_by_cached_key(|device| device.name.to_ascii_lowercase());
    devices.dedup_by(|left, right| left.id == right.id);
    Ok(devices)
}

unsafe fn default_capture_device_id(
    enumerator: &IMMDeviceEnumerator,
    role: ERole,
) -> Option<String> {
    unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, role) }
        .ok()
        .and_then(|device| unsafe { endpoint_device_id(&device).ok() })
}

fn capture_volume() -> Result<IAudioEndpointVolume> {
    unsafe {
        let enumerator = audio_device_enumerator()?;
        let device = capture_device(&enumerator)?;
        device
            .Activate(CLSCTX_ALL, None)
            .context("activate default capture endpoint volume")
    }
}

fn capture_volume_for_target(target: Option<&str>) -> Result<IAudioEndpointVolume> {
    let Some(device_id) = target else {
        return capture_volume();
    };
    let enumerator = audio_device_enumerator()?;
    let device_id = wide(device_id);
    let device = unsafe {
        enumerator
            .GetDevice(PCWSTR(device_id.as_ptr()))
            .context("get selected capture endpoint")?
    };
    unsafe {
        device
            .Activate(CLSCTX_ALL, None)
            .context("activate selected capture endpoint volume")
    }
}

unsafe fn capture_device(enumerator: &IMMDeviceEnumerator) -> Result<IMMDevice> {
    unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, eCommunications) }
        .or_else(|_| unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) })
        .context("get default communications capture endpoint")
}

fn set_all_capture_devices_mute(muted: bool) -> Result<()> {
    unsafe {
        let enumerator = audio_device_enumerator()?;
        let collection = enumerator
            .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
            .context("enumerate active capture endpoints")?;
        let count = collection
            .GetCount()
            .context("count active capture endpoints")?;
        anyhow::ensure!(count > 0, "no active capture endpoints are available");
        let mut first_error = None;
        let mut updated = 0_u32;

        for index in 0..count {
            let result = collection
                .Item(index)
                .context("get active capture endpoint")
                .and_then(|device| {
                    let volume: IAudioEndpointVolume = device
                        .Activate(CLSCTX_ALL, None)
                        .context("activate capture endpoint volume")?;
                    if !mute_change_required(volume.GetMute()?.as_bool(), muted) {
                        return Ok(false);
                    }
                    volume
                        .SetMute(muted, null())
                        .context("set capture endpoint mute")?;
                    Ok(true)
                });
            match result {
                Ok(true) => updated += 1,
                Ok(false) => {}
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if let Some(error) = first_error {
            return Err(error).with_context(|| {
                format!(
                    "set mute on every active capture endpoint ({updated} changed, {count} detected)"
                )
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod mute_action_tests {
    use super::*;

    #[test]
    fn explicit_mute_changes_only_when_the_requested_state_differs() {
        assert!(!mute_change_required(true, true));
        assert!(!mute_change_required(false, false));
        assert!(mute_change_required(false, true));
        assert!(mute_change_required(true, false));
    }
}

unsafe fn endpoint_device_id(device: &IMMDevice) -> Result<String> {
    let id = unsafe { device.GetId().context("get capture endpoint id")? };
    let value = unsafe { id.to_string().context("decode capture endpoint id")? };
    unsafe {
        CoTaskMemFree(Some(id.0 as *const c_void));
    }
    Ok(value)
}

unsafe fn endpoint_friendly_name(device: &IMMDevice) -> Result<String> {
    let store = unsafe {
        device
            .OpenPropertyStore(STGM_READ)
            .context("open capture endpoint properties")?
    };
    let value = unsafe {
        store
            .GetValue(&PKEY_Device_FriendlyName)
            .context("read capture endpoint friendly name")?
    };
    let mut buffer = [0_u16; 512];
    unsafe {
        PropVariantToString(&value, &mut buffer)
            .context("decode capture endpoint friendly name")?;
    }
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    let name = String::from_utf16_lossy(&buffer[..length]).trim().to_string();
    anyhow::ensure!(!name.is_empty(), "capture endpoint name is empty");
    Ok(name)
}
