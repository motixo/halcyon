//! Fixed, reviewed release downloads. PackageKit owns privileged installation.
use super::{Failure, packagekit};
use sha2::{Digest, Sha256};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{fs, io::Write, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::watch;

const DOWNLOAD_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Debug)]
pub struct Prepared {
    pub paths: Vec<PathBuf>,
    hashes: Vec<String>,
    // Keep verified files if the OS installer may still be reading them.
    pub handed_off: std::sync::atomic::AtomicBool,
    directory: PathBuf,
}
impl Drop for Prepared {
    fn drop(&mut self) {
        if !self.handed_off.load(std::sync::atomic::Ordering::Relaxed) {
            for path in &self.paths {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_dir(&self.directory);
        }
    }
}
impl Prepared {
    pub fn verify(&self) -> Result<(), Failure> {
        use std::io::Read;
        if self.paths.len() != 1 || self.hashes.len() != 1 {
            return Err(Failure::Untrusted);
        }
        for (path, hash) in self.paths.iter().zip(&self.hashes) {
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)
                .map_err(|_| Failure::Untrusted)?;
            let meta = file.metadata().map_err(|_| Failure::Untrusted)?;
            if !meta.is_file() || meta.len() > DOWNLOAD_LIMIT {
                return Err(Failure::Untrusted);
            }
            let mut bytes = Vec::new();
            file.take(DOWNLOAD_LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| Failure::Untrusted)?;
            if Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                != *hash
            {
                return Err(Failure::Untrusted);
            }
        }
        Ok(())
    }
}

pub async fn prepare(mut cancel: watch::Receiver<bool>) -> Result<Arc<Prepared>, Failure> {
    tokio::select! {
        biased;
        _ = packagekit::cancelled(&mut cancel) => Err(Failure::Cancelled),
        value = tokio::time::timeout(Duration::from_secs(90), download()) => value.unwrap_or(Err(Failure::Timeout)),
    }
}

fn redirect_allowed(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
pub(super) fn islands_version() -> Result<String, Failure> {
    let (_, assets) = islands_manifest(include_str!("../../../packaging/islands/download.json"))?;
    Ok(assets[0]
        .0
        .strip_prefix("cosmic-ext-halcyon-islands_")
        .and_then(|s| s.strip_suffix("_amd64.deb"))
        .ok_or(Failure::Untrusted)?
        .to_string())
}
fn islands_manifest(raw: &str) -> Result<(String, Vec<(String, String, usize)>), Failure> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| Failure::Untrusted)?;
    let version = value["package_version"]
        .as_str()
        .ok_or(Failure::Untrusted)?;
    let tag = value["tag"].as_str().ok_or(Failure::Untrusted)?;
    let hash = value["sha256"].as_str().ok_or(Failure::Untrusted)?;
    let bytes = value["bytes"].as_u64().ok_or(Failure::Untrusted)?;
    if value["format"] != "halcyon-islands-download"
        || value["version"] != 1
        || value["main_version"] != env!("CARGO_PKG_VERSION")
        || !super::identity::valid_version(version)
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".+~-".contains(&b))
        || tag != format!("v{version}")
        || hash.len() != 64
        || !hash.bytes().all(|b| b.is_ascii_hexdigit())
        || bytes == 0
        || bytes > DOWNLOAD_LIMIT
    {
        return Err(Failure::Untrusted);
    }
    Ok((
        tag.to_owned(),
        vec![(
            format!("cosmic-ext-halcyon-islands_{version}_amd64.deb"),
            hash.to_owned(),
            bytes as usize,
        )],
    ))
}
async fn download() -> Result<Arc<Prepared>, Failure> {
    let (tag, assets) = islands_manifest(include_str!("../../../packaging/islands/download.json"))?;
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|_| Failure::Failed)?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let directory = PathBuf::from(format!("/tmp/halcyon-updates-{nonce}"));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .map_err(|_| Failure::Failed)?;
    let mut prepared = Prepared {
        paths: Vec::new(),
        hashes: Vec::new(),
        directory,
        handed_off: std::sync::atomic::AtomicBool::new(false),
    };
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 4 && redirect_allowed(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("Untrusted download redirect")
            }
        }))
        .build()
        .map_err(|_| Failure::Failed)?;
    for (name, hash, limit) in assets {
        let url = format!("https://github.com/profilaz3/halcyon/releases/download/{tag}/{name}");
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                if e.is_redirect() {
                    Failure::Untrusted
                } else if e.is_timeout() {
                    Failure::Timeout
                } else {
                    Failure::Offline
                }
            })?
            .error_for_status()
            .map_err(|_| Failure::Offline)?;
        if response.content_length().is_some_and(|n| n != limit as u64) {
            return Err(Failure::Untrusted);
        }
        let mut bytes = Vec::with_capacity(limit);
        while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Offline)? {
            if bytes.len() + chunk.len() > limit {
                return Err(Failure::Untrusted);
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.len() != limit
            || Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                != hash
        {
            return Err(Failure::Untrusted);
        }
        let path = prepared.directory.join(name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)
            .map_err(|_| Failure::Failed)?;
        prepared.paths.push(path);
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| Failure::Failed)?;
        prepared.hashes.push(hash);
    }
    prepared.verify()?;
    Ok(Arc::new(prepared))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn islands_pin_is_reviewed_and_bound_to_main_version() {
        let raw = include_str!("../../../packaging/islands/download.json");
        let (tag, assets) = islands_manifest(raw).unwrap();
        let published: serde_json::Value =
            serde_json::from_str(include_str!("../../../scripts/apt/distribution.json")).unwrap();
        // A candidate release must be testable before its APT entry is published.
        // The release gate independently binds this pin to the final .deb bytes.
        // Once published, retain the additional archive cross-check here.
        if let Some(release) = published["releases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["tag"] == tag)
        {
            assert_eq!(release["assets"][&assets[0].0]["sha256"], assets[0].1);
            assert_eq!(release["assets"][&assets[0].0]["bytes"], assets[0].2);
        }
        for (field, bad) in [
            ("main_version", "wrong"),
            ("package_version", "../other"),
            ("tag", "other"),
            ("sha256", "bad"),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(raw).unwrap();
            value[field] = bad.into();
            assert_eq!(
                islands_manifest(&value.to_string()),
                Err(Failure::Untrusted)
            );
        }
    }
    #[test]
    fn download_redirects_are_restricted() {
        for url in [
            "http://github.com/x",
            "https://github.com.evil/x",
            "https://example.com/x",
            "https://user@github.com/x",
        ] {
            assert!(!redirect_allowed(&url.parse().unwrap()));
        }
        assert!(redirect_allowed(
            &"https://release-assets.githubusercontent.com/x?token=fixture"
                .parse()
                .unwrap()
        ));
    }
}

#[cfg(test)]
pub(super) fn fixture(directory: PathBuf) -> Prepared {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let mut p = Prepared {
        paths: vec![],
        hashes: vec![],
        directory,
        handed_off: std::sync::atomic::AtomicBool::new(false),
    };
    for name in ["islands.deb"] {
        let path = p.directory.join(name);
        fs::write(
            &path,
            b"private protocol fixture, not an installable package",
        )
        .unwrap();
        p.hashes.push(
            Sha256::digest(fs::read(&path).unwrap())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        );
        p.paths.push(path);
    }
    p
}
