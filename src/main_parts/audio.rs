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

    let volume = capture_volume()?;
    let next = !unsafe { volume.GetMute()? }.as_bool();
    unsafe {
        volume
            .SetMute(next, null())
            .context("toggle default microphone mute")?;
    }
    Ok(())
}

fn set_mute(target: Option<&str>, muted: bool) -> Result<()> {
    if is_all_microphones_target(target) {
        return set_all_capture_devices_mute(muted);
    }

    let volume = capture_volume()?;
    unsafe {
        volume
            .SetMute(muted, null())
            .context("set default microphone mute")?;
    }
    Ok(())
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

fn capture_volume() -> Result<IAudioEndpointVolume> {
    unsafe {
        let enumerator = audio_device_enumerator()?;
        let device = capture_device(&enumerator)?;
        device
            .Activate(CLSCTX_ALL, None)
            .context("activate default capture endpoint volume")
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
        let mut changed = 0_u32;

        for index in 0..count {
            let result = collection
                .Item(index)
                .context("get active capture endpoint")
                .and_then(|device| {
                    let volume: IAudioEndpointVolume = device
                        .Activate(CLSCTX_ALL, None)
                        .context("activate capture endpoint volume")?;
                    volume
                        .SetMute(muted, null())
                        .context("set capture endpoint mute")?;
                    Ok(())
                });
            match result {
                Ok(()) => changed += 1,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if let Some(error) = first_error {
            return Err(error).with_context(|| {
                format!(
                    "set mute on every active capture endpoint ({changed} of {count} succeeded)"
                )
            });
        }
        Ok(())
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
