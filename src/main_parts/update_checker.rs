#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateCheckTrigger {
    Automatic,
    Manual,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct UpdateCache {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_attempt_unix: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_successful_check_unix: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    available_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    release_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    download_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_notified_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UpdateStatusSnapshot {
    pub checking: bool,
    pub latest_release: String,
    pub last_successful_check: String,
    pub last_error: String,
    pub available_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AppVersion([u64; 3]);

#[derive(Debug)]
struct LatestRelease {
    version: String,
    release_url: String,
    download_url: Option<String>,
}

struct WinHttpHandle(*mut c_void);

impl WinHttpHandle {
    fn new(handle: *mut c_void, description: &str) -> Result<Self> {
        if handle.is_null() {
            return Err(windows::core::Error::from_win32())
                .with_context(|| description.to_string());
        }
        Ok(Self(handle))
    }
}

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}

struct UpdateCheckLock(HANDLE);

// The guard owns this mutex handle exclusively and transfers that ownership to
// the update worker. No other thread dereferences or closes the handle.
unsafe impl Send for UpdateCheckLock {}

struct UpdateCheckProgressGuard;

impl Drop for UpdateCheckProgressGuard {
    fn drop(&mut self) {
        UPDATE_CHECK_IN_PROGRESS.store(false, Ordering::Release);
    }
}

impl UpdateCheckLock {
    fn try_acquire() -> Result<Option<Self>> {
        let handle = unsafe { CreateMutexW(None, true, w!("MuteGuardUpdateCheck")) }
            .context("create update-check mutex")?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Ok(None);
        }
        Ok(Some(Self(handle)))
    }
}

impl Drop for UpdateCheckLock {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

pub(crate) fn schedule_automatic_update_check(enabled: bool) {
    if !enabled || !automatic_update_check_is_due() {
        return;
    }
    let _ = start_update_check(UpdateCheckTrigger::Automatic);
}

pub(crate) fn start_manual_update_check() -> bool {
    start_update_check(UpdateCheckTrigger::Manual)
}

pub(crate) fn update_check_in_progress() -> bool {
    if UPDATE_CHECK_IN_PROGRESS.load(Ordering::Acquire) {
        return true;
    }
    let Ok(handle) = (unsafe {
        OpenMutexW(
            MUTEX_MODIFY_STATE,
            false,
            w!("MuteGuardUpdateCheck"),
        )
    }) else {
        return false;
    };
    unsafe {
        let _ = CloseHandle(handle);
    }
    true
}

fn start_update_check(trigger: UpdateCheckTrigger) -> bool {
    if UPDATE_CHECK_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        return false;
    }

    let lock = match UpdateCheckLock::try_acquire() {
        Ok(Some(lock)) => lock,
        Ok(None) => {
            UPDATE_CHECK_IN_PROGRESS.store(false, Ordering::Release);
            return false;
        }
        Err(error) => {
            eprintln!("could not coordinate MuteGuard update check: {error:#}");
            UPDATE_CHECK_IN_PROGRESS.store(false, Ordering::Release);
            return false;
        }
    };

    match std::thread::Builder::new()
        .name("muteguard-update-check".to_string())
        .spawn(move || {
            let _progress = UpdateCheckProgressGuard;
            run_update_check(trigger);
            drop(lock);
        })
    {
        Ok(_) => true,
        Err(error) => {
            eprintln!("could not start MuteGuard update check: {error}");
            UPDATE_CHECK_IN_PROGRESS.store(false, Ordering::Release);
            false
        }
    }
}

fn run_update_check(trigger: UpdateCheckTrigger) {
    let now = unix_time_now();
    let mut cache = load_update_cache().unwrap_or_default();
    cache.last_attempt_unix = Some(now);

    match fetch_latest_release() {
        Ok(release) => {
            let available = is_newer_version(&release.version, env!("CARGO_PKG_VERSION"));
            cache.last_successful_check_unix = Some(now);
            cache.latest_version = Some(release.version.clone());
            cache.available_version = available.then_some(release.version);
            cache.release_url = Some(release.release_url);
            cache.download_url = release.download_url;
            cache.last_error = None;
        }
        Err(error) => {
            cache.last_error = Some(format!("{error:#}"));
        }
    }

    let notify = trigger == UpdateCheckTrigger::Automatic && should_notify_update(&cache);
    if notify {
        cache.last_notified_version.clone_from(&cache.available_version);
    }

    if let Err(error) = save_update_cache(&cache) {
        eprintln!("could not save MuteGuard update status: {error:#}");
        return;
    }

    if notify {
        let hwnd = STATE.lock().unwrap().hwnd;
        if !hwnd.0.is_null() {
            unsafe {
                let _ = PostMessageW(hwnd, WM_UPDATE_AVAILABLE, WPARAM(0), LPARAM(0));
            }
        }
    }
}

fn should_notify_update(cache: &UpdateCache) -> bool {
    cache.available_version.as_deref().is_some_and(|version| {
        is_available_version(version)
            && version_needs_notification(
                Some(version),
                cache.last_notified_version.as_deref(),
            )
    })
        && load_config()
            .map(|config| config.updates.check_automatically)
            .unwrap_or(false)
}

fn version_needs_notification(available: Option<&str>, last_notified: Option<&str>) -> bool {
    available.is_some() && available != last_notified
}

pub(crate) fn notify_available_update() {
    let Ok(cache) = load_update_cache() else {
        return;
    };
    let Some(version) = cache.available_version.clone() else {
        return;
    };
    if !is_available_version(&version) {
        return;
    }
    let launch_url = preferred_update_url(&cache);
    show_update_notification(
        &format!("MuteGuard {version} is available"),
        "Click to download the installer from the official GitHub release.",
        &launch_url,
    );
}

pub(crate) fn open_available_update() -> Result<()> {
    let cache = load_update_cache().context("read update status")?;
    let available_version = cache.available_version.as_deref();
    anyhow::ensure!(
        available_version.is_some_and(is_available_version),
        "no newer MuteGuard release is available"
    );
    open_external_update_url(&preferred_update_url(&cache))
}

pub(crate) fn update_status_snapshot() -> UpdateStatusSnapshot {
    match load_update_cache() {
        Ok(cache) => {
            let available_version = cache
                .available_version
                .filter(|version| is_available_version(version));
            UpdateStatusSnapshot {
                checking: update_check_in_progress(),
                latest_release: cache.latest_version.as_ref().map_or_else(
                    || "Not checked".to_string(),
                    |version| {
                        if available_version.as_ref() == Some(version) {
                            format!("{version} available")
                        } else {
                            format!("{version} (up to date)")
                        }
                    },
                ),
                last_successful_check: cache
                    .last_successful_check_unix
                    .map_or_else(|| "Never".to_string(), relative_time_label),
                last_error: cache.last_error.unwrap_or_else(|| "None".to_string()),
                available_version,
            }
        }
        Err(error) => UpdateStatusSnapshot {
            checking: update_check_in_progress(),
            latest_release: "Not checked".to_string(),
            last_successful_check: "Never".to_string(),
            last_error: format!("Could not read local update status: {error:#}"),
            available_version: None,
        },
    }
}

fn automatic_update_check_is_due() -> bool {
    load_update_cache()
        .ok()
        .and_then(|cache| cache.last_attempt_unix)
        .is_none_or(|last_attempt| update_check_is_due(last_attempt, unix_time_now()))
}

fn update_check_is_due(last_attempt: u64, now: u64) -> bool {
    last_attempt > now || now - last_attempt >= UPDATE_CHECK_INTERVAL_SECS
}

fn load_update_cache() -> Result<UpdateCache> {
    let path = update_cache_path()?;
    if !path.exists() {
        return Ok(UpdateCache::default());
    }
    let contents = fs::read_to_string(&path).context("read update cache")?;
    serde_json::from_str(&contents).context("parse update cache")
}

fn save_update_cache(cache: &UpdateCache) -> Result<()> {
    let path = update_cache_path()?;
    let contents = serde_json::to_vec_pretty(cache).context("serialize update cache")?;
    write_file_atomically(&path, &contents, "MuteGuard update cache")
}

fn update_cache_path() -> Result<PathBuf> {
    Ok(app_config_dir()?.join("update-cache.json"))
}

fn fetch_latest_release() -> Result<LatestRelease> {
    let response = winhttp_get_github_release()?;
    let release: GitHubRelease =
        serde_json::from_slice(&response).context("parse GitHub release response")?;
    latest_release_from_github(release)
}

fn winhttp_get_github_release() -> Result<Vec<u8>> {
    unsafe {
        let session = WinHttpHandle::new(
            WinHttpOpen(
                w!("MuteGuard update checker"),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR(null()),
                PCWSTR(null()),
                0,
            ),
            "open WinHTTP session",
        )?;
        WinHttpSetTimeouts(session.0, 5_000, 5_000, 5_000, 10_000)
            .context("set update request timeouts")?;

        let host = wide(UPDATE_API_HOST);
        let connection = WinHttpHandle::new(
            WinHttpConnect(session.0, PCWSTR(host.as_ptr()), 443, 0),
            "connect to GitHub",
        )?;
        let path = wide(UPDATE_API_PATH);
        let request = WinHttpHandle::new(
            WinHttpOpenRequest(
                connection.0,
                w!("GET"),
                PCWSTR(path.as_ptr()),
                PCWSTR(null()),
                PCWSTR(null()),
                null(),
                WINHTTP_FLAG_SECURE,
            ),
            "create GitHub update request",
        )?;
        let headers = wide_without_nul(concat!(
            "Accept: application/vnd.github+json\r\n",
            "X-GitHub-Api-Version: 2022-11-28\r\n"
        ));
        WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0)
            .context("send GitHub update request")?;
        WinHttpReceiveResponse(request.0, null_mut()).context("receive GitHub update response")?;

        let mut status_code = 0_u32;
        let mut status_size = size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR(null()),
            Some((&mut status_code as *mut u32).cast()),
            &mut status_size,
            null_mut(),
        )
        .context("read GitHub response status")?;
        anyhow::ensure!(status_code == 200, "GitHub returned HTTP {status_code}");

        let mut response = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let mut bytes_read = 0_u32;
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut bytes_read,
            )
            .context("read GitHub update response")?;
            if bytes_read == 0 {
                break;
            }
            anyhow::ensure!(
                response.len() + bytes_read as usize <= UPDATE_RESPONSE_LIMIT_BYTES,
                "GitHub update response exceeded the safety limit"
            );
            response.extend_from_slice(&buffer[..bytes_read as usize]);
        }
        Ok(response)
    }
}

fn latest_release_from_github(release: GitHubRelease) -> Result<LatestRelease> {
    let version = parse_app_version(&release.tag_name)
        .with_context(|| format!("unsupported GitHub release tag {}", release.tag_name))?;
    let normalized_version = format_app_version(&version);
    let release_url = if is_allowed_release_url(&release.html_url) {
        release.html_url
    } else {
        format!("{UPDATE_RELEASE_BASE_URL}/latest")
    };
    let expected_asset = format!("muteguard-{normalized_version}-windows-x64-setup.exe");
    let download_url = release
        .assets
        .into_iter()
        .find(|asset| asset.name == expected_asset)
        .map(|asset| asset.browser_download_url)
        .filter(|url| is_allowed_download_url(url));

    Ok(LatestRelease {
        version: normalized_version,
        release_url,
        download_url,
    })
}

fn parse_app_version(value: &str) -> Option<AppVersion> {
    let value = value.trim().strip_prefix(['v', 'V']).unwrap_or(value.trim());
    let mut parts = value.split('.');
    let version = AppVersion([
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ]);
    parts.next().is_none().then_some(version)
}

fn format_app_version(version: &AppVersion) -> String {
    format!("{}.{}.{}", version.0[0], version.0[1], version.0[2])
}

fn is_newer_version(candidate: &str, current: &str) -> bool {
    parse_app_version(candidate).zip(parse_app_version(current)).is_some_and(
        |(candidate, current)| candidate > current,
    )
}

fn is_available_version(candidate: &str) -> bool {
    is_newer_version(candidate, env!("CARGO_PKG_VERSION"))
}

fn preferred_update_url(cache: &UpdateCache) -> String {
    cache
        .download_url
        .as_deref()
        .filter(|url| is_allowed_download_url(url))
        .or_else(|| {
            cache
                .release_url
                .as_deref()
                .filter(|url| is_allowed_release_url(url))
        })
        .unwrap_or("https://github.com/Minus193/MuteGuard/releases/latest")
        .to_string()
}

fn is_allowed_release_url(url: &str) -> bool {
    url.starts_with("https://github.com/Minus193/MuteGuard/releases/")
}

fn is_allowed_download_url(url: &str) -> bool {
    url.starts_with("https://github.com/Minus193/MuteGuard/releases/download/")
}

fn open_external_update_url(url: &str) -> Result<()> {
    anyhow::ensure!(
        is_allowed_release_url(url) || is_allowed_download_url(url),
        "refusing to open an unexpected update URL"
    );
    let url = wide(url);
    let result = unsafe {
        ShellExecuteW(
            HWND(null_mut()),
            w!("open"),
            PCWSTR(url.as_ptr()),
            PCWSTR(null()),
            PCWSTR(null()),
            SW_SHOWNORMAL,
        )
    };
    anyhow::ensure!(
        result.0 as isize > 32,
        "Windows could not open the update link"
    );
    Ok(())
}

fn wide_without_nul(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn unix_time_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn relative_time_label(timestamp: u64) -> String {
    let elapsed = unix_time_now().saturating_sub(timestamp);
    match elapsed {
        0..=59 => "Less than a minute ago".to_string(),
        60..=3_599 => relative_unit_label(elapsed / 60, "minute"),
        3_600..=86_399 => relative_unit_label(elapsed / 3_600, "hour"),
        _ => relative_unit_label(elapsed / 86_400, "day"),
    }
}

fn relative_unit_label(value: u64, unit: &str) -> String {
    let suffix = if value == 1 { "" } else { "s" };
    format!("{value} {unit}{suffix} ago")
}

#[cfg(test)]
mod update_checker_tests {
    use super::*;

    fn release(tag: &str, asset_name: &str, asset_url: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            html_url: format!("{UPDATE_RELEASE_BASE_URL}/tag/{tag}"),
            assets: vec![GitHubReleaseAsset {
                name: asset_name.to_string(),
                browser_download_url: asset_url.to_string(),
            }],
        }
    }

    #[test]
    fn versions_are_compared_numerically() {
        assert!(is_newer_version("v1.10.0", "1.9.9"));
        assert!(!is_newer_version("1.4.0", "1.4.0"));
        assert!(!is_newer_version("1.3.9", "1.4.0"));
        assert!(!is_newer_version("1.4.0-beta", "1.3.0"));
    }

    #[test]
    fn cached_versions_at_or_below_the_installed_version_are_not_available() {
        assert!(!is_available_version(env!("CARGO_PKG_VERSION")));
        assert!(!is_available_version("1.3.2"));
        assert!(is_available_version("1.5.2"));
    }

    #[test]
    fn only_the_exact_x64_installer_is_selected() {
        let url = "https://github.com/Minus193/MuteGuard/releases/download/v1.4.0/muteguard-1.4.0-windows-x64-setup.exe";
        let latest = latest_release_from_github(release(
            "v1.4.0",
            "muteguard-1.4.0-windows-x64-setup.exe",
            url,
        ))
        .unwrap();

        assert_eq!(latest.version, "1.4.0");
        assert_eq!(latest.download_url.as_deref(), Some(url));
    }

    #[test]
    fn unexpected_download_hosts_are_rejected() {
        let latest = latest_release_from_github(release(
            "v1.4.0",
            "muteguard-1.4.0-windows-x64-setup.exe",
            "https://example.com/untrusted.exe",
        ))
        .unwrap();

        assert!(latest.download_url.is_none());
        assert!(is_allowed_release_url(&latest.release_url));
    }

    #[test]
    fn notification_is_emitted_only_once_per_available_version() {
        assert!(version_needs_notification(Some("1.4.0"), None));
        assert!(!version_needs_notification(Some("1.4.0"), Some("1.4.0")));
        assert!(version_needs_notification(
            Some("1.5.0"),
            Some("1.4.0")
        ));
        assert!(!version_needs_notification(None, Some("1.4.0")));
    }

    #[test]
    fn daily_interval_handles_boundaries_and_clock_rollback() {
        assert!(!update_check_is_due(1_000, 1_000));
        assert!(!update_check_is_due(
            1_000,
            1_000 + UPDATE_CHECK_INTERVAL_SECS - 1
        ));
        assert!(update_check_is_due(
            1_000,
            1_000 + UPDATE_CHECK_INTERVAL_SECS
        ));
        assert!(update_check_is_due(2_000, 1_000));
    }

    #[test]
    fn relative_time_units_use_singular_and_plural_labels() {
        assert_eq!(relative_unit_label(1, "hour"), "1 hour ago");
        assert_eq!(relative_unit_label(2, "hour"), "2 hours ago");
    }
}
