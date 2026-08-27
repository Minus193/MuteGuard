const FEEDBACK_SAMPLE_RATE: u32 = 44_100;
const FEEDBACK_DURATION_SECS: f64 = 0.11;
pub(crate) const MAX_CUSTOM_SOUND_BYTES: usize = 12 * 1024 * 1024;
const MAX_CUSTOM_SOUND_MILLIS: u64 = 5_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FeedbackKind {
    Mute,
    Unmute,
}

impl FeedbackKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Mute => "Mute",
            Self::Unmute => "Unmute",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WaveInfo {
    data_start: usize,
    data_len: usize,
    duration_millis: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SoundPlaybackRequest {
    kind: FeedbackKind,
    volume: u8,
    use_custom: bool,
}

struct SoundFeedbackQueue {
    pending: Arc<(Mutex<Option<SoundPlaybackRequest>>, Condvar)>,
}

static SOUND_FEEDBACK_QUEUE: LazyLock<Option<SoundFeedbackQueue>> = LazyLock::new(|| {
    SoundFeedbackQueue::start()
        .map_err(|error| eprintln!("failed to start sound feedback worker: {error}"))
        .ok()
});

impl SoundFeedbackQueue {
    fn start() -> std::io::Result<Self> {
        let pending = Arc::new((Mutex::new(None), Condvar::new()));
        let worker_pending = Arc::clone(&pending);
        std::thread::Builder::new()
            .name("muteguard-sound-feedback".to_string())
            .spawn(move || sound_feedback_worker(&worker_pending))?;
        Ok(Self { pending })
    }

    fn submit(&self, request: SoundPlaybackRequest) {
        let (pending, ready) = &*self.pending;
        pending.lock().unwrap().replace(request);
        ready.notify_one();
    }
}

fn sound_feedback_worker(pending: &Arc<(Mutex<Option<SoundPlaybackRequest>>, Condvar)>) {
    loop {
        let request = {
            let (pending, ready) = &**pending;
            let mut request = pending.lock().unwrap();
            while request.is_none() {
                request = ready.wait(request).unwrap();
            }
            request.take().unwrap()
        };
        play_feedback_request(request);
    }
}

fn play_sound_feedback(muted: bool, settings: &SoundFeedbackSettings) {
    if !settings.enabled || settings.volume == 0 {
        return;
    }
    play_feedback(
        if muted {
            FeedbackKind::Mute
        } else {
            FeedbackKind::Unmute
        },
        settings,
    );
}

pub(crate) fn preview_sound_feedback(kind: FeedbackKind, settings: &SoundFeedbackSettings) {
    if settings.volume == 0 {
        return;
    }
    if !request_background_sound_preview(kind) {
        queue_sound_preview(kind, settings);
    }
}

fn play_feedback(kind: FeedbackKind, settings: &SoundFeedbackSettings) {
    queue_sound_preview(kind, settings);
}

fn queue_sound_preview(kind: FeedbackKind, settings: &SoundFeedbackSettings) {
    let volume = settings.volume.min(100);
    let use_custom = match kind {
        FeedbackKind::Mute => settings.mute_source == "Custom",
        FeedbackKind::Unmute => settings.unmute_source == "Custom",
    };
    let Some(queue) = SOUND_FEEDBACK_QUEUE.as_ref() else {
        report_runtime_error(
            "MuteGuard could not play sound feedback",
            "The sound feedback worker could not be started.",
        );
        return;
    };
    queue.submit(SoundPlaybackRequest {
        kind,
        volume,
        use_custom,
    });
}

fn play_feedback_request(request: SoundPlaybackRequest) {
    if request.use_custom {
        match load_custom_sound(request.kind, request.volume) {
            Ok(wave) => {
                if play_wave_sync(&wave) {
                    return;
                }
                eprintln!("custom sound playback failed; using the built-in tone");
            }
            Err(error) => eprintln!("custom sound is unavailable; using the built-in tone: {error:#}"),
        }
    }

    let wave = synthesize_feedback_wave(request.kind == FeedbackKind::Mute, request.volume);
    if !play_wave_sync(&wave) {
        report_runtime_error(
            "MuteGuard could not play sound feedback",
            "Windows rejected both the selected sound and the built-in tone.",
        );
    }
}

fn play_wave_sync(wave: &[u8]) -> bool {
    let mut aligned = vec![0_u32; wave.len().div_ceil(size_of::<u32>())];
    unsafe {
        std::ptr::copy_nonoverlapping(
            wave.as_ptr(),
            aligned.as_mut_ptr().cast::<u8>(),
            wave.len(),
        );
        PlaySoundW(
            PCWSTR(aligned.as_ptr().cast()),
            HMODULE::default(),
            SND_MEMORY | SND_NODEFAULT | SND_SYNC | SND_SYSTEM,
        )
        .as_bool()
    }
}

fn load_custom_sound(kind: FeedbackKind, volume: u8) -> Result<Vec<u8>> {
    let mut wave = read_custom_sound(kind)?;
    let info = validate_custom_sound(&wave)?;
    scale_pcm_samples(&mut wave, info, volume);
    Ok(wave)
}

pub(crate) fn save_custom_sound(kind: FeedbackKind, bytes: &[u8]) -> Result<u64> {
    let info = validate_custom_sound(bytes)?;

    let path = custom_sound_path(kind)?;
    write_file_atomically(&path, bytes, "custom feedback sound")?;
    Ok(info.duration_millis)
}

pub(crate) fn custom_sound_available(kind: FeedbackKind) -> bool {
    read_custom_sound(kind).is_ok_and(|bytes| validate_custom_sound(&bytes).is_ok())
}

fn custom_sound_path(kind: FeedbackKind) -> Result<PathBuf> {
    let file_name = match kind {
        FeedbackKind::Mute => "mute.wav",
        FeedbackKind::Unmute => "unmute.wav",
    };
    Ok(app_config_dir()?.join("sounds").join(file_name))
}

fn read_custom_sound(kind: FeedbackKind) -> Result<Vec<u8>> {
    let path = custom_sound_path(kind)?;
    let file_size = fs::metadata(&path)
        .context("inspect custom feedback sound")?
        .len();
    anyhow::ensure!(
        file_size <= MAX_CUSTOM_SOUND_BYTES as u64,
        "custom feedback sound exceeds the file size limit"
    );
    fs::read(path).context("read custom feedback sound")
}

fn validate_custom_sound(bytes: &[u8]) -> Result<WaveInfo> {
    anyhow::ensure!(
        bytes.len() <= MAX_CUSTOM_SOUND_BYTES,
        "The WAV file is too large. Choose a file no longer than 5 seconds."
    );
    let info = analyze_pcm_wave(bytes)?;
    anyhow::ensure!(
        info.duration_millis <= MAX_CUSTOM_SOUND_MILLIS,
        "The WAV file is {:.2} seconds long. The maximum is 5 seconds.",
        info.duration_millis as f64 / 1_000.0
    );
    Ok(info)
}

fn analyze_pcm_wave(bytes: &[u8]) -> Result<WaveInfo> {
    anyhow::ensure!(bytes.len() >= 44, "The selected file is not a valid WAV file.");
    anyhow::ensure!(
        &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "The selected file is not a RIFF/WAVE file."
    );
    let declared_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    anyhow::ensure!(
        declared_size
            .checked_add(8)
            .is_some_and(|size| size <= bytes.len()),
        "The WAV file is truncated."
    );

    let mut offset = 12_usize;
    let mut format = None;
    let mut data = None;
    while offset
        .checked_add(8)
        .is_some_and(|end| end <= bytes.len())
    {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_len =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start
            .checked_add(chunk_len)
            .filter(|end| *end <= bytes.len())
            .context("The WAV file contains a truncated chunk.")?;
        if chunk_id == b"fmt " && chunk_len >= 16 {
            let audio_format =
                u16::from_le_bytes(bytes[chunk_start..chunk_start + 2].try_into().unwrap());
            let channels =
                u16::from_le_bytes(bytes[chunk_start + 2..chunk_start + 4].try_into().unwrap());
            let sample_rate = u32::from_le_bytes(
                bytes[chunk_start + 4..chunk_start + 8]
                    .try_into()
                    .unwrap(),
            );
            let byte_rate = u32::from_le_bytes(
                bytes[chunk_start + 8..chunk_start + 12]
                    .try_into()
                    .unwrap(),
            );
            let block_align = u16::from_le_bytes(
                bytes[chunk_start + 12..chunk_start + 14]
                    .try_into()
                    .unwrap(),
            );
            let bits_per_sample = u16::from_le_bytes(
                bytes[chunk_start + 14..chunk_start + 16]
                    .try_into()
                    .unwrap(),
            );
            format = Some((
                audio_format,
                channels,
                sample_rate,
                byte_rate,
                block_align,
                bits_per_sample,
            ));
        } else if chunk_id == b"data" {
            data = Some((chunk_start, chunk_len));
        }
        offset = chunk_end + (chunk_len & 1);
    }

    let (audio_format, channels, sample_rate, byte_rate, block_align, bits_per_sample) =
        format.context("The WAV file has no format chunk.")?;
    anyhow::ensure!(
        audio_format == 1 && bits_per_sample == 16 && channels > 0 && sample_rate > 0,
        "Custom sounds must be uncompressed 16-bit PCM WAV files."
    );
    let expected_block_align = channels
        .checked_mul(bits_per_sample / 8)
        .context("The WAV channel layout is invalid.")?;
    let expected_byte_rate = sample_rate
        .checked_mul(u32::from(expected_block_align))
        .context("The WAV sample rate is invalid.")?;
    anyhow::ensure!(
        block_align == expected_block_align && byte_rate == expected_byte_rate,
        "The WAV format metadata is inconsistent."
    );
    let (data_start, data_len) = data.context("The WAV file has no audio data.")?;
    anyhow::ensure!(
        data_len > 0 && data_len % usize::from(block_align) == 0,
        "The WAV audio data is invalid."
    );
    Ok(WaveInfo {
        data_start,
        data_len,
        duration_millis: (data_len as u64 * 1_000).div_ceil(u64::from(byte_rate)),
    })
}

fn scale_pcm_samples(wave: &mut [u8], info: WaveInfo, volume: u8) {
    let gain = f32::from(volume.min(100)) / 100.0;
    let data = &mut wave[info.data_start..info.data_start + info.data_len];
    for sample in data.as_chunks_mut::<2>().0 {
        let value = i16::from_le_bytes([sample[0], sample[1]]);
        sample.copy_from_slice(&((f32::from(value) * gain).round() as i16).to_le_bytes());
    }
}

fn synthesize_feedback_wave(muted: bool, volume: u8) -> Vec<u8> {
    let sample_count = (FEEDBACK_SAMPLE_RATE as f64 * FEEDBACK_DURATION_SECS).round() as usize;
    let data_size = sample_count * size_of::<i16>();
    let mut wave = Vec::with_capacity(44 + data_size);
    wave.extend_from_slice(b"RIFF");
    wave.extend_from_slice(&(36_u32 + data_size as u32).to_le_bytes());
    wave.extend_from_slice(b"WAVEfmt ");
    wave.extend_from_slice(&16_u32.to_le_bytes());
    wave.extend_from_slice(&1_u16.to_le_bytes());
    wave.extend_from_slice(&1_u16.to_le_bytes());
    wave.extend_from_slice(&FEEDBACK_SAMPLE_RATE.to_le_bytes());
    wave.extend_from_slice(&(FEEDBACK_SAMPLE_RATE * 2).to_le_bytes());
    wave.extend_from_slice(&2_u16.to_le_bytes());
    wave.extend_from_slice(&16_u16.to_le_bytes());
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&(data_size as u32).to_le_bytes());

    let volume = (f64::from(volume) / 100.0).powf(1.15) * 0.55;
    let (start_frequency, end_frequency) = if muted {
        (690.0, 430.0)
    } else {
        (470.0, 740.0)
    };
    let attack_samples = (FEEDBACK_SAMPLE_RATE as f64 * 0.012) as usize;
    let release_samples = (FEEDBACK_SAMPLE_RATE as f64 * 0.028) as usize;
    let mut phase = 0.0_f64;

    for index in 0..sample_count {
        let progress = index as f64 / sample_count.saturating_sub(1).max(1) as f64;
        let frequency = start_frequency + (end_frequency - start_frequency) * progress;
        phase += std::f64::consts::TAU * frequency / FEEDBACK_SAMPLE_RATE as f64;
        let attack = (index as f64 / attack_samples.max(1) as f64).min(1.0);
        let release = ((sample_count - index) as f64 / release_samples.max(1) as f64).min(1.0);
        let envelope = attack.min(release);
        let sample = (phase.sin() * envelope * volume * f64::from(i16::MAX)).round() as i16;
        wave.extend_from_slice(&sample.to_le_bytes());
    }
    wave
}

#[cfg(test)]
mod sound_feedback_tests {
    use super::*;

    #[test]
    fn pending_sound_feedback_keeps_only_the_latest_request() {
        let pending = Arc::new((Mutex::new(None), Condvar::new()));
        let queue = SoundFeedbackQueue {
            pending: Arc::clone(&pending),
        };
        let mute = SoundPlaybackRequest {
            kind: FeedbackKind::Mute,
            volume: 40,
            use_custom: false,
        };
        let unmute = SoundPlaybackRequest {
            kind: FeedbackKind::Unmute,
            volume: 60,
            use_custom: true,
        };

        queue.submit(mute);
        queue.submit(unmute);

        assert_eq!(*pending.0.lock().unwrap(), Some(unmute));
    }

    #[test]
    fn generated_feedback_is_a_complete_pcm_wave() {
        let wave = synthesize_feedback_wave(false, 50);
        let info = analyze_pcm_wave(&wave).unwrap();
        assert_eq!(&wave[..4], b"RIFF");
        assert_eq!(info.data_start, 44);
        assert!(info.duration_millis < 1_000);
        assert!(wave[44..].iter().any(|sample| *sample != 0));
    }

    #[test]
    fn custom_sound_duration_is_read_from_its_pcm_data() {
        let wave = synthesize_feedback_wave(true, 50);
        assert_eq!(analyze_pcm_wave(&wave).unwrap().duration_millis, 110);
    }

    #[test]
    fn zero_volume_produces_silence() {
        assert!(synthesize_feedback_wave(true, 0)[44..]
            .iter()
            .all(|sample| *sample == 0));
    }

    #[test]
    fn default_feedback_volume_is_audible_without_clipping() {
        let wave = synthesize_feedback_wave(false, default_sound_feedback_volume());
        let peak = wave[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]).unsigned_abs())
            .max()
            .unwrap();

        assert!(peak > 5_000);
        assert!(peak < i16::MAX as u16);
    }

    #[test]
    fn custom_sound_validation_rejects_audio_longer_than_five_seconds() {
        let data_len = FEEDBACK_SAMPLE_RATE as usize * size_of::<i16>() * 6;
        let mut wave = synthesize_feedback_wave(true, 50);
        wave.resize(44 + data_len, 0);
        wave[4..8].copy_from_slice(&(36_u32 + data_len as u32).to_le_bytes());
        wave[40..44].copy_from_slice(&(data_len as u32).to_le_bytes());

        let error = validate_custom_sound(&wave).unwrap_err().to_string();
        assert!(error.contains("maximum is 5 seconds"));
    }

    #[test]
    fn custom_sound_validation_rejects_inconsistent_pcm_metadata() {
        let mut wave = synthesize_feedback_wave(false, 50);
        wave[28..32].copy_from_slice(&1_u32.to_le_bytes());

        let error = validate_custom_sound(&wave).unwrap_err().to_string();
        assert!(error.contains("metadata is inconsistent"));
    }

    #[test]
    fn custom_sound_validation_rejects_a_truncated_riff_container() {
        let mut wave = synthesize_feedback_wave(false, 50);
        wave[4..8].copy_from_slice(&u32::MAX.to_le_bytes());

        let error = validate_custom_sound(&wave).unwrap_err().to_string();
        assert!(error.contains("truncated"));
    }
}
