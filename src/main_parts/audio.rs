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
