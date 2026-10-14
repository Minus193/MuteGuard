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

#[derive(Clone, Copy, Default)]
pub(crate) struct ApoEffects {
    pub gate: bool,
    pub compressor: bool,
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
    Ok(ApoEffects {
        gate: content.lines().any(|line| line.trim() == "Gate=1"),
        compressor: content.lines().any(|line| line.trim() == "Compressor=1"),
    })
}

pub(crate) fn write_apo_effects(effects: ApoEffects) -> anyhow::Result<()> {
    let path = apo_effects_path()?;
    anyhow::ensure!(path.parent().is_some_and(std::path::Path::exists), "APO is not installed");
    std::fs::write(path, format!(
        "[Effects]\nGate={}\nCompressor={}\n",
        u8::from(effects.gate), u8::from(effects.compressor)
    ))?;
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
    use super::capture_endpoint_guid;

    #[test]
    fn extracts_capture_endpoint_guid() {
        assert_eq!(
            capture_endpoint_guid("{0.0.1.00000000}.{ae68b532-cdb6-4253-8b86-0c6137188f7d}"),
            Some("{ae68b532-cdb6-4253-8b86-0c6137188f7d}")
        );
        assert_eq!(capture_endpoint_guid("not an endpoint"), None);
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
