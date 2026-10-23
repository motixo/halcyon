//! Automatic, minimal daily activity reporting. No content or hardware identity.
//! Only the desktop process invokes `run`; Settings and fixtures never do.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

const RETRY_SECONDS: i64 = 6 * 60 * 60;
const WARMUP: Duration = Duration::from_secs(300);
const COUNTRY_ENDPOINT: &str = "https://api.country.is/";
const ENDPOINT: &str = include_str!("../assets/usage-endpoint.txt");
pub const DISCLOSURE: &str = "Halcyon reports a random installation ID, app version and approximate first country to Google daily while running. Country.is looks up the country from your public IP; Halcyon does not store the IP. No widget content is sent.";

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    installation: String,
    last_success: Option<String>,
    retry_after: i64,
    #[serde(default)]
    country: Option<String>,
}
#[derive(Serialize)]
struct Report<'a> {
    schema: u8,
    installation: &'a str,
    version: &'a str,
    country: Option<&'a str>,
}
#[derive(Deserialize)]
struct Acknowledgement {
    ok: bool,
    day: Option<String>,
}
fn token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "Could not create reporting identity")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn valid_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
struct Lock(std::fs::File);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
fn lock(path: &Path) -> Result<Lock, String> {
    let parent = path.parent().ok_or("Missing state directory")?;
    if !parent.exists() {
        std::fs::create_dir_all(parent).map_err(|_| "Could not create state directory")?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "Could not protect state directory")?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path.with_extension("lock"))
        .map_err(|_| "Could not lock activity state")?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("Activity state is busy".into());
    }
    Ok(Lock(file))
}
fn load(path: &Path) -> Result<State, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(State::default()),
        Err(_) => return Err("Could not read activity state".into()),
    };
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read activity state")?;
    if bytes.len() > 4096 {
        return Err("Invalid activity state".into());
    }
    let state: State = serde_json::from_slice(&bytes).map_err(|_| "Invalid activity state")?;
    if !valid_token(&state.installation)
        || state.country.as_deref().is_some_and(|c| !valid_country(c))
    {
        return Err("Invalid activity identity".into());
    }
    Ok(state)
}
fn save(path: &Path, state: &State) -> Result<(), String> {
    let tmp = path.with_extension(format!("{}.tmp", token()?));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|_| "Could not save activity state")?;
        file.write_all(&serde_json::to_vec(state).map_err(|_| "Invalid activity state")?)
            .map_err(|_| "Could not write activity state")?;
        file.sync_all()
            .map_err(|_| "Could not save activity state")?;
        std::fs::rename(&tmp, path).map_err(|_| "Could not replace activity state")
    })();
    let _ = std::fs::remove_file(tmp);
    result.map_err(String::from)
}
fn claim(path: &Path, day: &str, now: i64) -> Result<Option<String>, String> {
    let _lock = lock(path)?;
    let mut state = load(path)?;
    if state.last_success.as_deref() == Some(day) || now < state.retry_after {
        return Ok(None);
    }
    if state.installation.is_empty() {
        state.installation = token()?;
    }
    // Reserve before networking, so crashes/restarts/second processes respect backoff.
    state.retry_after = now.saturating_add(RETRY_SECONDS);
    save(path, &state)?;
    Ok(Some(state.installation))
}
fn complete(path: &Path, installation: &str, day: &str) -> Result<(), String> {
    let _lock = lock(path)?;
    let mut state = load(path)?;
    if state.installation != installation {
        return Ok(());
    }
    state.last_success = Some(day.into());
    state.retry_after = 0;
    save(path, &state)
}
fn valid_country(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|b| b.is_ascii_uppercase())
}
fn remember_country(
    path: &Path,
    installation: &str,
    country: &str,
) -> Result<Option<String>, String> {
    if !valid_country(country) {
        return Ok(None);
    }
    let _lock = lock(path)?;
    let mut state = load(path)?;
    if state.installation != installation {
        return Ok(None);
    }
    if state.country.is_none() {
        state.country = Some(country.into());
        save(path, &state)?;
    }
    Ok(state.country)
}
async fn country_for(path: &Path, installation: &str, url: &str) -> Option<String> {
    let state = load(path).ok()?;
    if state.installation != installation {
        return None;
    }
    if let Some(country) = state.country {
        return Some(country);
    }
    // Separate short request: never attach the installation token, follow redirects,
    // retain the returned IP address, or let lookup failure stop an activity report.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;
    let mut response = client.get(url).send().await.ok()?;
    if response.status() != reqwest::StatusCode::OK {
        return None;
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if body.len() + chunk.len() > 1024 {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    #[derive(Deserialize)]
    struct Country {
        country: String,
    }
    let country: Country = serde_json::from_slice(&body).ok()?;
    remember_country(path, installation, &country.country)
        .ok()
        .flatten()
}
async fn report_once(
    path: &Path,
    client: &reqwest::Client,
    url: reqwest::Url,
    installation: &str,
    country_url: &str,
) -> Result<String, String> {
    let country = country_for(path, installation, country_url).await;
    send(client, url, installation, country.as_deref()).await
}
fn endpoint(value: &str) -> Option<reqwest::Url> {
    let url = reqwest::Url::parse(value.trim()).ok()?;
    (url.scheme() == "https"
        && url.host_str() == Some("script.google.com")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().starts_with("/macros/s/")
        && url.path().ends_with("/exec")
        && url.path().split('/').count() == 5
        && url.path().split('/').nth(3).is_some_and(|id| {
            !id.is_empty()
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        }))
    .then_some(url)
}
fn client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            // Apps Script serves the response from a temporary Google URL.
            if attempt.previous().len() < 4
                && attempt.url().scheme() == "https"
                && matches!(
                    attempt.url().host_str(),
                    Some("script.google.com" | "script.googleusercontent.com")
                )
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
}
async fn send(
    client: &reqwest::Client,
    url: reqwest::Url,
    installation: &str,
    country: Option<&str>,
) -> Result<String, String> {
    let mut response = client
        .post(url)
        .json(&Report {
            schema: 2,
            installation,
            version: env!("CARGO_PKG_VERSION"),
            country,
        })
        .send()
        .await
        .map_err(|_| "Daily report unavailable")?;
    if response.status() != reqwest::StatusCode::OK {
        return Err("Daily report not accepted".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Incomplete acknowledgement")?
    {
        if body.len() + chunk.len() > 1024 {
            return Err("Invalid acknowledgement".into());
        }
        body.extend_from_slice(&chunk);
    }
    let ack: Acknowledgement =
        serde_json::from_slice(&body).map_err(|_| "Invalid acknowledgement")?;
    let day = ack
        .day
        .filter(|d| d.len() == 10 && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok());
    if !ack.ok {
        return Err("Daily report not accepted".into());
    }
    day.ok_or("Invalid acknowledgement".into())
}
/// Only production desktop processes report; previews and test builds never do.
pub fn available() -> bool {
    !cfg!(debug_assertions)
        && !cfg!(test)
        && endpoint(ENDPOINT).is_some()
        && ![
            "DESKTOP_CARDS_CONFIG",
            "DESKTOP_CARDS_DESKTOP_ROOT",
            "DESKTOP_CARDS_CAPTURE_DIR",
            "DESKTOP_CARDS_SETTINGS_CAPTURE_DIR",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}
pub fn start(path: Option<PathBuf>) {
    if !available() {
        return;
    }
    if let Some(path) = path {
        let _ = std::thread::Builder::new()
            .name("halcyon-activity".into())
            .spawn(move || {
                if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    runtime.block_on(run(path));
                }
            });
    }
}
async fn run(path: PathBuf) {
    if !available() {
        return;
    }
    let Some(url) = endpoint(ENDPOINT) else {
        return;
    };
    let Ok(client) = client() else {
        return;
    };
    tokio::time::sleep(WARMUP).await;
    loop {
        let now = Utc::now();
        let day = now.format("%Y-%m-%d").to_string();
        if let Ok(Some(installation)) = claim(&path, &day, now.timestamp()) {
            if let Ok(server_day) =
                report_once(&path, &client, url.clone(), &installation, COUNTRY_ENDPOINT).await
            {
                // Use the locally attempted day to avoid repeated successes on a clock offset;
                // the server always assigns/deduplicates the actual UTC day.
                let _ = server_day;
                let _ = complete(&path, &installation, &day);
            }
        }
        // Small local checks; no queues or historical backfill after being offline.
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
#[cfg(test)]
#[path = "usage/tests.rs"]
mod tests;
