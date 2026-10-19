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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum EffectModule {
    HighPass,
    Noise,
    Gate,
    Expander,
    DeEsser,
    Eq,
    Compressor,
    Limiter,
}

impl EffectModule {
    const DEFAULT_ORDER: [Self; 8] = [
        Self::HighPass,
        Self::Noise,
        Self::Gate,
        Self::Expander,
        Self::DeEsser,
        Self::Eq,
        Self::Compressor,
        Self::Limiter,
    ];

    fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::HighPass),
            1 => Some(Self::Noise),
            2 => Some(Self::Gate),
            3 => Some(Self::Expander),
            4 => Some(Self::DeEsser),
            5 => Some(Self::Eq),
            6 => Some(Self::Compressor),
            7 => Some(Self::Limiter),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ApoEffects {
    pub order: [EffectModule; 8],
    pub high_pass: bool,
    pub noise: bool,
    pub gate: bool,
    pub expander: bool,
    pub de_esser: bool,
    pub eq: bool,
    pub compressor: bool,
    pub limiter: bool,
    pub high_pass_hz: i32,
    pub high_pass_slope: i32,
    pub noise_strength: i32,
    pub gate_threshold_db: i32,
    pub gate_hold_ms: i32,
    pub gate_release_ms: i32,
    pub expander_threshold_db: i32,
    pub expander_ratio_tenths: i32,
    pub expander_attack_ms: i32,
    pub expander_release_ms: i32,
    pub de_esser_hz: i32,
    pub de_esser_threshold_db: i32,
    pub de_esser_max_reduction_db: i32,
    pub de_esser_ratio_tenths: i32,
    pub de_esser_attack_ms: i32,
    pub de_esser_release_ms: i32,
    pub eq_low_hz: i32,
    pub eq_low_gain_db: i32,
    pub eq_low_q_tenths: i32,
    pub eq_mid_hz: i32,
    pub eq_mid_gain_db: i32,
    pub eq_mid_q_tenths: i32,
    pub eq_high_hz: i32,
    pub eq_high_gain_db: i32,
    pub eq_high_q_tenths: i32,
    pub compressor_threshold_db: i32,
    pub compressor_ratio_tenths: i32,
    pub compressor_attack_ms: i32,
    pub compressor_release_ms: i32,
    pub limiter_threshold_db: i32,
    pub limiter_release_ms: i32,
}

impl Default for ApoEffects {
    fn default() -> Self {
        Self {
            order: EffectModule::DEFAULT_ORDER,
            high_pass: false,
            noise: false,
            gate: false,
            expander: false,
            de_esser: false,
            eq: false,
            compressor: false,
            limiter: false,
            high_pass_hz: 80,
            high_pass_slope: 12,
            noise_strength: 100,
            gate_threshold_db: -45,
            gate_hold_ms: 100,
            gate_release_ms: 150,
            expander_threshold_db: -40,
            expander_ratio_tenths: 20,
            expander_attack_ms: 10,
            expander_release_ms: 50,
            de_esser_hz: 6000,
            de_esser_threshold_db: -30,
            de_esser_max_reduction_db: 6,
            de_esser_ratio_tenths: 40,
            de_esser_attack_ms: 2,
            de_esser_release_ms: 60,
            eq_low_hz: 150,
            eq_low_gain_db: 0,
            eq_low_q_tenths: 10,
            eq_mid_hz: 3000,
            eq_mid_gain_db: 0,
            eq_mid_q_tenths: 10,
            eq_high_hz: 8000,
            eq_high_gain_db: 0,
            eq_high_q_tenths: 10,
            compressor_threshold_db: -18,
            compressor_ratio_tenths: 25,
            compressor_attack_ms: 10,
            compressor_release_ms: 150,
            limiter_threshold_db: -6,
            limiter_release_ms: 60,
        }
    }
}

impl ApoEffects {
    pub(crate) fn clamped(mut self) -> Self {
        let mut seen = [false; 8];
        for module in self.order {
            let id = module as usize;
            if seen[id] {
                self.order = EffectModule::DEFAULT_ORDER;
                break;
            }
            seen[id] = true;
        }
        self.high_pass_hz = self.high_pass_hz.clamp(40, 300);
        self.high_pass_slope = if self.high_pass_slope == 24 { 24 } else { 12 };
        self.noise_strength = self.noise_strength.clamp(0, 100);
        self.gate_threshold_db = self.gate_threshold_db.clamp(-80, -10);
        self.gate_hold_ms = self.gate_hold_ms.clamp(0, 500);
        self.gate_release_ms = self.gate_release_ms.clamp(10, 1000);
        self.expander_threshold_db = self.expander_threshold_db.clamp(-80, -10);
        self.expander_ratio_tenths = self.expander_ratio_tenths.clamp(10, 100);
        self.expander_attack_ms = self.expander_attack_ms.clamp(1, 100);
        self.expander_release_ms = self.expander_release_ms.clamp(10, 1000);
        self.de_esser_hz = self.de_esser_hz.clamp(3000, 10000);
        self.de_esser_threshold_db = self.de_esser_threshold_db.clamp(-60, -5);
        self.de_esser_max_reduction_db = self.de_esser_max_reduction_db.clamp(1, 18);
        self.de_esser_ratio_tenths = self.de_esser_ratio_tenths.clamp(10, 100);
        self.de_esser_attack_ms = self.de_esser_attack_ms.clamp(1, 100);
        self.de_esser_release_ms = self.de_esser_release_ms.clamp(10, 1000);
        self.eq_low_hz = self.eq_low_hz.clamp(60, 500);
        self.eq_low_gain_db = self.eq_low_gain_db.clamp(-12, 12);
        self.eq_low_q_tenths = self.eq_low_q_tenths.clamp(3, 100);
        self.eq_mid_hz = self.eq_mid_hz.clamp(500, 6000);
        self.eq_mid_gain_db = self.eq_mid_gain_db.clamp(-12, 12);
        self.eq_mid_q_tenths = self.eq_mid_q_tenths.clamp(3, 100);
        self.eq_high_hz = self.eq_high_hz.clamp(4000, 12000);
        self.eq_high_gain_db = self.eq_high_gain_db.clamp(-12, 12);
        self.eq_high_q_tenths = self.eq_high_q_tenths.clamp(3, 100);
        self.compressor_threshold_db = self.compressor_threshold_db.clamp(-40, 0);
        self.compressor_ratio_tenths = self.compressor_ratio_tenths.clamp(10, 100);
        self.compressor_attack_ms = self.compressor_attack_ms.clamp(1, 100);
        self.compressor_release_ms = self.compressor_release_ms.clamp(20, 1000);
        self.limiter_threshold_db = self.limiter_threshold_db.clamp(-20, -1);
        self.limiter_release_ms = self.limiter_release_ms.clamp(10, 1000);
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ApoEffects::default());
        }
        Err(error) => return Err(error.into()),
    };
    Ok(parse_apo_effects(&content))
}

fn parse_apo_effects(content: &str) -> ApoEffects {
    let value = |key: &str, fallback: i32| {
        content
            .lines()
            .find_map(|line| {
                let (name, raw) = line.trim().split_once('=')?;
                (name.trim() == key)
                    .then(|| raw.trim().parse::<i32>().ok())
                    .flatten()
            })
            .unwrap_or(fallback)
    };
    let mut order = EffectModule::DEFAULT_ORDER;
    for (index, module) in order.iter_mut().enumerate() {
        let Some(parsed) = EffectModule::from_id(value(&format!("Order{index}"), *module as i32))
        else {
            order = EffectModule::DEFAULT_ORDER;
            break;
        };
        *module = parsed;
    }
    ApoEffects {
        order,
        high_pass: value("HighPass", 0) == 1,
        noise: value("Noise", 0) == 1,
        gate: value("Gate", 0) == 1,
        expander: value("Expander", 0) == 1,
        de_esser: value("DeEsser", 0) == 1,
        eq: value("Eq", 0) == 1,
        compressor: value("Compressor", 0) == 1,
        limiter: value("Limiter", 0) == 1,
        high_pass_hz: value("HighPassHz", 80),
        high_pass_slope: value("HighPassSlope", 12),
        noise_strength: value("NoiseStrength", 100),
        gate_threshold_db: value("GateThresholdDb", -45),
        gate_hold_ms: value("GateHoldMs", 100),
        gate_release_ms: value("GateReleaseMs", 150),
        expander_threshold_db: value("ExpanderThresholdDb", -40),
        expander_ratio_tenths: value("ExpanderRatioTenths", 20),
        expander_attack_ms: value("ExpanderAttackMs", 10),
        expander_release_ms: value("ExpanderReleaseMs", 50),
        de_esser_hz: value("DeEsserHz", 6000),
        de_esser_threshold_db: value("DeEsserThresholdDb", -30),
        de_esser_max_reduction_db: value("DeEsserMaxReductionDb", 6),
        de_esser_ratio_tenths: value("DeEsserRatioTenths", 40),
        de_esser_attack_ms: value("DeEsserAttackMs", 2),
        de_esser_release_ms: value("DeEsserReleaseMs", 60),
        eq_low_hz: value("EqLowHz", 150),
        eq_low_gain_db: value("EqLowGainDb", 0),
        eq_low_q_tenths: value("EqLowQTenths", 10),
        eq_mid_hz: value("EqMidHz", 3000),
        eq_mid_gain_db: value("EqMidGainDb", 0),
        eq_mid_q_tenths: value("EqMidQTenths", 10),
        eq_high_hz: value("EqHighHz", 8000),
        eq_high_gain_db: value("EqHighGainDb", 0),
        eq_high_q_tenths: value("EqHighQTenths", 10),
        compressor_threshold_db: value("CompressorThresholdDb", -18),
        compressor_ratio_tenths: value("CompressorRatioTenths", 25),
        compressor_attack_ms: value("CompressorAttackMs", 10),
        compressor_release_ms: value("CompressorReleaseMs", 150),
        limiter_threshold_db: value("LimiterThresholdDb", -6),
        limiter_release_ms: value("LimiterReleaseMs", 60),
    }
    .clamped()
}

pub(crate) fn write_apo_effects(effects: ApoEffects) -> anyhow::Result<()> {
    let path = apo_effects_path()?;
    anyhow::ensure!(
        path.parent().is_some_and(std::path::Path::exists),
        "APO is not installed"
    );
    std::fs::write(path, format_apo_effects(effects.clamped()))?;
    Ok(())
}

fn format_apo_effects(effects: ApoEffects) -> String {
    use std::fmt::Write as _;
    let mut content = String::from("[Effects]\n");
    for (key, value) in [
        ("HighPass", i32::from(effects.high_pass)),
        ("Noise", i32::from(effects.noise)),
        ("Gate", i32::from(effects.gate)),
        ("Expander", i32::from(effects.expander)),
        ("DeEsser", i32::from(effects.de_esser)),
        ("Eq", i32::from(effects.eq)),
        ("Compressor", i32::from(effects.compressor)),
        ("Limiter", i32::from(effects.limiter)),
        ("HighPassHz", effects.high_pass_hz),
        ("HighPassSlope", effects.high_pass_slope),
        ("NoiseStrength", effects.noise_strength),
        ("GateThresholdDb", effects.gate_threshold_db),
        ("GateHoldMs", effects.gate_hold_ms),
        ("GateReleaseMs", effects.gate_release_ms),
        ("ExpanderThresholdDb", effects.expander_threshold_db),
        ("ExpanderRatioTenths", effects.expander_ratio_tenths),
        ("ExpanderAttackMs", effects.expander_attack_ms),
        ("ExpanderReleaseMs", effects.expander_release_ms),
        ("DeEsserHz", effects.de_esser_hz),
        ("DeEsserThresholdDb", effects.de_esser_threshold_db),
        ("DeEsserMaxReductionDb", effects.de_esser_max_reduction_db),
        ("DeEsserRatioTenths", effects.de_esser_ratio_tenths),
        ("DeEsserAttackMs", effects.de_esser_attack_ms),
        ("DeEsserReleaseMs", effects.de_esser_release_ms),
        ("EqLowHz", effects.eq_low_hz),
        ("EqLowGainDb", effects.eq_low_gain_db),
        ("EqLowQTenths", effects.eq_low_q_tenths),
        ("EqMidHz", effects.eq_mid_hz),
        ("EqMidGainDb", effects.eq_mid_gain_db),
        ("EqMidQTenths", effects.eq_mid_q_tenths),
        ("EqHighHz", effects.eq_high_hz),
        ("EqHighGainDb", effects.eq_high_gain_db),
        ("EqHighQTenths", effects.eq_high_q_tenths),
        ("CompressorThresholdDb", effects.compressor_threshold_db),
        ("CompressorRatioTenths", effects.compressor_ratio_tenths),
        ("CompressorAttackMs", effects.compressor_attack_ms),
        ("CompressorReleaseMs", effects.compressor_release_ms),
        ("LimiterThresholdDb", effects.limiter_threshold_db),
        ("LimiterReleaseMs", effects.limiter_release_ms),
    ] {
        writeln!(content, "{key}={value}").expect("writing to String cannot fail");
    }
    for (index, module) in effects.order.into_iter().enumerate() {
        writeln!(content, "Order{index}={}", module as u8).expect("writing to String cannot fail");
    }
    content
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
    apo_effects_path().is_ok_and(|path| path.with_file_name("live-test-state.json").is_file())
}

pub(crate) fn run_apo_test_action(install: bool, device_id: &str) -> anyhow::Result<()> {
    use windows::Win32::{
        Foundation::WAIT_OBJECT_0,
        System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_HIDE,
        },
    };

    let directory = apo_test_directory()?;
    let script = directory.join("live-test.ps1");
    anyhow::ensure!(script.is_file(), "APO test script is unavailable");
    if install {
        anyhow::ensure!(
            directory.join("x64/Release/MuteGuardAPO.dll").is_file(),
            "APO DLL is unavailable"
        );
        anyhow::ensure!(
            directory.join("rnnoise/weights_blob.bin").is_file(),
            "RNNoise model is unavailable"
        );
    }
    let action = if install { "Install" } else { "Restore" };
    let endpoint_argument = if install {
        let guid =
            capture_endpoint_guid(device_id).context("microphone endpoint ID is unavailable")?;
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
    anyhow::ensure!(
        !info.hProcess.is_invalid(),
        "Windows did not start the APO action"
    );
    let result = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    let mut exit_code = 1;
    let exit_result = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess)? };
    anyhow::ensure!(
        result == WAIT_OBJECT_0,
        "Windows could not wait for the APO action"
    );
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
        "MuteGuard APO active".to_string()
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
        && guid[1..37]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-'))
    .then_some(guid)
}

#[cfg(test)]
mod effect_status_tests {
    use super::{
        ApoEffects, EffectModule, capture_endpoint_guid, format_apo_effects, parse_apo_effects,
    };

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
        assert!(
            !settings.high_pass
                && !settings.expander
                && !settings.de_esser
                && !settings.eq
                && !settings.limiter
        );
        assert_eq!(settings.order, EffectModule::DEFAULT_ORDER);
        assert_eq!(settings.noise_strength, 100);
        assert_eq!(settings.gate_threshold_db, -45);
        assert_eq!(settings.compressor_ratio_tenths, 25);
    }

    #[test]
    fn effect_values_are_bounded_before_saving() {
        let settings =
            parse_apo_effects("NoiseStrength=400\nGateThresholdDb=-200\nCompressorRatioTenths=0\n");
        assert_eq!(settings.noise_strength, 100);
        assert_eq!(settings.gate_threshold_db, -80);
        assert_eq!(settings.compressor_ratio_tenths, 10);
    }

    #[test]
    fn effect_order_and_values_round_trip() {
        let mut settings = ApoEffects::default();
        settings.order.swap(0, 7);
        settings.high_pass = true;
        settings.high_pass_hz = 100;
        settings.expander = true;
        settings.eq_mid_gain_db = 3;
        settings.eq_mid_q_tenths = 7;
        settings.de_esser_ratio_tenths = 30;
        settings.de_esser_attack_ms = 5;
        settings.de_esser_release_ms = 80;
        let restored = parse_apo_effects(&format_apo_effects(settings));
        assert_eq!(restored.order, settings.order);
        assert!(restored.high_pass && restored.expander);
        assert_eq!(restored.high_pass_hz, 100);
        assert_eq!(restored.eq_mid_gain_db, 3);
        assert_eq!(restored, settings);
    }

    #[test]
    fn invalid_effect_order_uses_safe_default() {
        let settings = parse_apo_effects("Order0=7\nOrder1=7\nHighPassSlope=19\n");
        assert_eq!(settings.order, EffectModule::DEFAULT_ORDER);
        assert_eq!(settings.high_pass_slope, 12);
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
    let count = unsafe {
        collection
            .GetCount()
            .context("count active capture endpoints")?
    };
    let mut devices = Vec::with_capacity(count as usize);
    for index in 0..count {
        let Ok(device) = (unsafe { collection.Item(index) }) else {
            continue;
        };
        let Ok(id) = (unsafe { endpoint_device_id(&device) }) else {
            continue;
        };
        let name =
            unsafe { endpoint_friendly_name(&device) }.unwrap_or_else(|_| "Microphone".to_string());
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
    let name = String::from_utf16_lossy(&buffer[..length])
        .trim()
        .to_string();
    anyhow::ensure!(!name.is_empty(), "capture endpoint name is empty");
    Ok(name)
}
