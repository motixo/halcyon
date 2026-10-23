//! Local evidence only. No PATH lookup, APT invocation, network, or D-Bus.
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const MAIN: &str = "cosmic-ext-halcyon";
const EXE: &str = "/usr/bin/halcyon";
const SOURCE: &str = "/etc/apt/sources.list.d/halcyon.sources";
const PREFS: &str = "/etc/apt/preferences.d/halcyon";
const KEY: &str = "/usr/share/keyrings/halcyon-archive-keyring.gpg";
const STORE: &str = "com.system76.CosmicStore.desktop";
const SOURCE_TEXT: &str = "Types: deb\nURIs: https://profilaz3.github.io/halcyon/apt\nSuites: noble\nComponents: main\nArchitectures: amd64\nSigned-By: /usr/share/keyrings/halcyon-archive-keyring.gpg\n";
const PREFS_TEXT: &str = "Package: cosmic-ext-halcyon cosmic-ext-halcyon-islands halcyon-archive-keyring halcyon-update-source\nPin: release o=Halcyon,l=Halcyon,a=noble,n=noble,c=main\nPin-Priority: 500\n\nPackage: *\nPin: release o=Halcyon\nPin-Priority: -1\n\nPackage: *\nPin: origin \"profilaz3.github.io\"\nPin-Priority: -1\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateChannel {
    Apt,
    ManualDeb,
    Source,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub format_version: u32,
    pub channel: UpdateChannel,
    pub reason: &'static str,
    pub running_version: String,
    pub installed_version: Option<String>,
    pub architecture: Option<String>,
    pub store: Option<PathBuf>,
    // Private evidence binds pre/post transaction checks to actual measured bytes.
    pub(super) evidence: Option<String>,
}
impl Default for Identity {
    fn default() -> Self {
        Self {
            format_version: 1,
            channel: UpdateChannel::Unknown,
            reason: "Installation could not be verified.",
            running_version: env!("CARGO_PKG_VERSION").into(),
            installed_version: None,
            architecture: None,
            store: None,
            evidence: None,
        }
    }
}
impl Identity {
    pub fn can_check(&self) -> bool {
        self.format_version == 1 && self.channel == UpdateChannel::Apt && self.evidence.is_some()
    }
    pub fn same_installation(&self, other: &Self) -> bool {
        self.can_check()
            && other.can_check()
            && self.evidence == other.evidence
            && self.installed_version == other.installed_version
            && self.architecture == other.architecture
    }
}
pub(super) fn isolated() -> bool {
    [
        "DESKTOP_CARDS_CONFIG",
        "DESKTOP_CARDS_DESKTOP_ROOT",
        "DESKTOP_CARDS_LAYOUT_EVIDENCE",
        "DESKTOP_CARDS_SETTINGS_CAPTURE",
    ]
    .iter()
    .any(|key| std::env::var_os(key).is_some())
        || cfg!(test)
}
pub(super) async fn inspect() -> Identity {
    if isolated() {
        return Identity::default();
    }
    tokio::time::timeout(
        Duration::from_secs(12),
        tokio::task::spawn_blocking(|| {
            let mut identity = measure(
                Path::new("/"),
                Path::new("/proc/self/exe"),
                Path::new(env!("HALCYON_BUILD_BINARY")),
            );
            identity.store = store_entry(
                &application_dirs(),
                Path::new("/usr/bin/gio"),
                Path::new("/usr/bin/cosmic-store"),
            );
            identity
        }),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or_default()
}
fn rooted(root: &Path, absolute: &str) -> PathBuf {
    root.join(absolute.trim_start_matches('/'))
}
fn read(path: &Path, max: u64) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut value = String::new();
    file.take(max + 1).read_to_string(&mut value).ok()?;
    (value.len() as u64 <= max).then_some(value)
}
fn digest(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > 256 * 1024 * 1024 {
        return None;
    }
    let mut hash = Sha256::new();
    let mut reader = file.take(256 * 1024 * 1024 + 1);
    let mut buffer = [0u8; 65536];
    loop {
        let n = reader.read(&mut buffer).ok()?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Some(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn stamp(path: &Path) -> Option<(u64, u64, u64, i64, i64)> {
    let m = fs::metadata(path).ok()?;
    Some((m.dev(), m.ino(), m.len(), m.ctime(), m.ctime_nsec()))
}
// A small fixed-output utility, always off the UI thread. No shell or PATH.
fn md5(path: &Path) -> Option<String> {
    let mut child = Command::new("/usr/bin/md5sum")
        .arg("--")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(5))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&out.stdout).ok()?;
    let hash = text.split_whitespace().next()?;
    (hash.len() == 32 && hash.bytes().all(|c| c.is_ascii_hexdigit())).then(|| hash.to_owned())
}
fn fields(text: &str) -> Option<BTreeMap<String, String>> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut previous = String::new();
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            map.get_mut(&previous)?
                .push_str(&format!("\n{}", line.trim()));
        } else {
            let (key, value) = line.split_once(':')?;
            if map.insert(key.into(), value.trim().into()).is_some() {
                return None;
            }
            previous = key.into();
        }
    }
    Some(map)
}
struct Database {
    packages: BTreeMap<String, BTreeMap<String, String>>,
    raw: String,
}
impl Database {
    fn load(root: &Path) -> Option<Self> {
        let raw = read(&rooted(root, "/var/lib/dpkg/status"), 32 * 1024 * 1024)?;
        let mut packages = BTreeMap::new();
        for stanza in raw.trim().split("\n\n") {
            let f = fields(stanza)?;
            let name = f.get("Package")?;
            if [MAIN, "halcyon-update-source", "halcyon-archive-keyring"].contains(&name.as_str())
                && packages.insert(name.clone(), f).is_some()
            {
                return None;
            }
        }
        Some(Self { packages, raw })
    }
    fn installed(&self, name: &str, arch: &str) -> Option<&BTreeMap<String, String>> {
        let p = self.packages.get(name)?;
        (p.get("Status")?
            .split_whitespace()
            .skip(1)
            .eq(["ok", "installed"])
            && p.get("Architecture")? == arch)
            .then_some(p)
    }
    fn record(&self, root: &Path, name: &str, suffix: &str) -> Option<String> {
        let p = self.packages.get(name)?;
        let arch = p.get("Architecture")?;
        let prefix = rooted(root, "/var/lib/dpkg/info");
        // dpkg uses an architecture suffix for Multi-Arch packages.
        let base = if prefix.join(format!("{name}.list")).is_file() {
            name.to_owned()
        } else {
            format!("{name}:{arch}")
        };
        read(&prefix.join(format!("{base}.{suffix}")), 4 * 1024 * 1024)
    }
    fn verified_file(&self, root: &Path, name: &str, path: &str) -> Option<String> {
        let p = self.packages.get(name)?;
        let list = self.record(root, name, "list")?;
        if list.lines().filter(|v| *v == path).count() != 1 {
            return None;
        }
        let target = rooted(root, path);
        let meta = fs::symlink_metadata(&target).ok()?;
        if !meta.is_file()
            || (root == Path::new("/") && (meta.uid() != 0 || meta.mode() & 0o022 != 0))
        {
            return None;
        }
        let before = stamp(&target)?;
        let expected = self
            .record(root, name, "md5sums")
            .and_then(|records| {
                let rows: Vec<_> = records
                    .lines()
                    .filter_map(|line| line.split_once("  "))
                    .filter(|(_, file)| *file == path.trim_start_matches('/'))
                    .map(|(hash, _)| hash.to_owned())
                    .collect();
                (rows.len() == 1).then(|| rows[0].clone())
            })
            .or_else(|| {
                let rows: Vec<_> = p
                    .get("Conffiles")?
                    .lines()
                    .map(|line| line.split_whitespace().collect::<Vec<_>>())
                    .filter(|row| row.len() == 2 && row[0] == path)
                    .collect();
                (rows.len() == 1).then(|| rows[0][1].to_owned())
            })?;
        if md5(&target)? != expected {
            return None;
        }
        let sha = digest(&target)?;
        (stamp(&target)? == before).then_some(sha)
    }
}
fn enrollment(root: &Path, db: &Database) -> Option<Option<String>> {
    let source = rooted(root, SOURCE);
    // Conflicting copies cannot silently broaden the scoped source contract.
    let mut paths = vec![
        rooted(root, "/etc/apt/sources.list"),
        rooted(root, "/etc/apt/preferences"),
    ];
    for dir in ["/etc/apt/sources.list.d", "/etc/apt/preferences.d"] {
        let directory = rooted(root, dir);
        if !directory.try_exists().ok()? {
            continue;
        }
        for entry in fs::read_dir(directory).ok()? {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            let apt_reads = if dir.ends_with("sources.list.d") {
                name.ends_with(".list") || name.ends_with(".sources")
            } else {
                name.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                    || name.ends_with(".pref")
            };
            if apt_reads && path != source && path != rooted(root, PREFS) {
                paths.push(path);
            }
        }
    }
    for path in paths {
        if !path.try_exists().ok()? {
            continue;
        }
        let text = read(&path, 1024 * 1024)?;
        let other = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        if other.contains("profilaz3.github.io")
            || other.contains("Halcyon")
            || other.contains("halcyon")
        {
            return None;
        }
    }
    if !source.try_exists().ok()? {
        return Some(None);
    }
    let text = read(&source, 65536)?;
    let parsed = fields(text.trim())?;
    if parsed.get("Enabled").is_some_and(|s| s == "no") {
        return Some(None);
    }
    // Deliberately accept only the owned bootstrap contract, not a lookalike URL
    // or PackageKit's unauthenticated package-id data field.
    if text != SOURCE_TEXT || read(&rooted(root, PREFS), 65536)? != PREFS_TEXT {
        return None;
    }
    db.installed("halcyon-update-source", "all")?;
    db.installed("halcyon-archive-keyring", "all")?;
    let mut hashes = String::new();
    for (package, path) in [
        ("halcyon-update-source", SOURCE),
        ("halcyon-update-source", PREFS),
        ("halcyon-archive-keyring", KEY),
    ] {
        hashes.push_str(&db.verified_file(root, package, path)?);
    }
    if fs::metadata(rooted(root, KEY)).ok()?.len() < 64 {
        return None;
    }
    Some(Some(hashes))
}
pub(super) fn measure(root: &Path, proc_exe: &Path, source_binary: &Path) -> Identity {
    let mut result = Identity::default();
    let Some(target) = fs::read_link(proc_exe).ok() else {
        return result;
    };
    let deleted = target
        .as_os_str()
        .as_encoded_bytes()
        .ends_with(b" (deleted)");
    let installed_path = rooted(root, EXE);
    let Some(running_hash) = digest(proc_exe) else {
        return result;
    };
    let normalized = target
        .to_str()
        .and_then(|p| p.strip_suffix(" (deleted)"))
        .map(PathBuf::from)
        .unwrap_or_else(|| target.clone());
    if deleted {
        result.reason = "The running app was replaced or removed. Restart Halcyon to recheck.";
    }
    if normalized != installed_path {
        if deleted {
            result.reason = "The running app was replaced or removed. Restart Halcyon to recheck.";
        } else if target == source_binary
            && stamp(proc_exe) == stamp(source_binary)
            && digest(source_binary).as_ref() == Some(&running_hash)
        {
            result.channel = UpdateChannel::Source;
            result.reason = "Local source build · update from your source checkout.";
        }
        return result;
    }
    let Some(db) = Database::load(root) else {
        return result;
    };
    let Some(package) = db.installed(MAIN, "amd64") else {
        return result;
    };
    let Some(version) = package.get("Version").filter(|v| valid_version(v)) else {
        return result;
    };
    if read(&rooted(root, "/var/lib/dpkg/diversions"), 4 * 1024 * 1024)
        .is_some_and(|v| v.lines().any(|v| v == EXE))
    {
        return result;
    }
    let Some(installed_hash) = db.verified_file(root, MAIN, EXE) else {
        return result;
    };
    result.installed_version = Some(version.clone());
    result.architecture = Some("amd64".into());
    if deleted || running_hash != installed_hash || stamp(proc_exe) != stamp(&installed_path) {
        result.reason = "The running app was replaced or removed. Restart Halcyon to recheck.";
        return result;
    }
    match enrollment(root, &db) {
        Some(Some(enrollment)) => {
            result.channel = UpdateChannel::Apt;
            result.reason = "System updates · checks only when you ask.";
            result.evidence = Some(format!("{running_hash}{enrollment}"));
        }
        Some(None) => {
            result.channel = UpdateChannel::ManualDeb;
            result.reason = "Manual package · system updates are not enabled.";
        }
        None => result.reason = "System update enrollment could not be verified.",
    }
    // Refuse a dpkg transaction racing this read, even if its file operations
    // happened in between our checks. Re-measure before and after every check.
    if read(&rooted(root, "/var/lib/dpkg/status"), 32 * 1024 * 1024).as_ref() != Some(&db.raw)
        || digest(proc_exe).as_ref() != Some(&running_hash)
        || digest(&installed_path).as_ref() != Some(&running_hash)
    {
        result.channel = UpdateChannel::Unknown;
        result.evidence = None;
        result.reason = "Installation changed. Reopen Settings to check it again.";
    }
    result
}
pub(super) fn valid_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let version = if let Some((epoch, version)) = value.split_once(':') {
        if epoch.is_empty() || !epoch.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
        version
    } else {
        value
    };
    if version.is_empty() || !version.as_bytes()[0].is_ascii_digit() {
        return false;
    }
    if let Some((upstream, revision)) = version.rsplit_once('-') {
        if upstream.is_empty()
            || revision.is_empty()
            || !revision
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"+.~".contains(&c))
        {
            return false;
        }
    }
    version
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || b".+:~-".contains(&c))
}
fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::data_dir() {
        dirs.push(home.join("applications"));
    }
    dirs.extend(
        std::env::var_os("XDG_DATA_DIRS")
            .unwrap_or_else(|| "/usr/local/share:/usr/share".into())
            .to_string_lossy()
            .split(':')
            .filter(|s| s.starts_with('/'))
            .map(|s| Path::new(s).join("applications")),
    );
    dirs
}
pub(super) fn store_entry(dirs: &[PathBuf], launcher: &Path, program: &Path) -> Option<PathBuf> {
    let executable =
        |path: &Path| fs::metadata(path).is_ok_and(|m| m.is_file() && m.mode() & 0o111 != 0);
    if !executable(launcher) || !executable(program) {
        return None;
    }
    for dir in dirs {
        let path = dir.join(STORE);
        if !path.try_exists().ok()? {
            continue;
        }
        let text = read(&path, 65536)?;
        if text
            .lines()
            .filter(|line| line.trim() == "[Desktop Entry]")
            .count()
            != 1
        {
            return None;
        }
        let section = text
            .split("[Desktop Entry]")
            .nth(1)?
            .split('\n')
            .skip(1)
            .take_while(|l| !l.starts_with('['));
        let mut entries = BTreeMap::new();
        for line in section {
            if let Some((key, value)) = line.split_once('=') {
                if entries.insert(key.trim(), value.trim()).is_some() {
                    return None;
                }
            }
        }
        return (entries.get("Type") == Some(&"Application")
            && matches!(
                entries.get("Exec"),
                Some(&"cosmic-store %u") | Some(&"/usr/bin/cosmic-store %u")
            )
            && entries.get("DBusActivatable") != Some(&"true")
            && entries.get("Hidden") != Some(&"true")
            && entries.get("Terminal") != Some(&"true"))
        .then_some(path);
    }
    None
}
pub(super) async fn open_store() -> Result<(), String> {
    if isolated() {
        return Err("COSMIC Store is unavailable in this preview.".into());
    }
    let identity = inspect().await;
    let entry = identity
        .store
        .as_ref()
        .filter(|_| identity.can_check())
        .ok_or("COSMIC Store is unavailable for this installation.")?;
    // Launch the resolved, verified desktop ID through the desktop launcher,
    // with no URI/search argument and no invented Updates action.
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::process::Command::new("/usr/bin/gio")
            .arg("launch")
            .arg(entry)
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status(),
    )
    .await;
    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        _ => Err("Couldn’t open COSMIC Store. Open it from Applications.".into()),
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
