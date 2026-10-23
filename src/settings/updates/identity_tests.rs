use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "halcyon-identity-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let f = Self { root };
        f.put(EXE, b"fixture app");
        f.put(SOURCE, SOURCE_TEXT.as_bytes());
        f.put(PREFS, PREFS_TEXT.as_bytes());
        f.put(KEY, &[0x99; 256]);
        f.put("/var/lib/dpkg/status",b"Package: cosmic-ext-halcyon\nStatus: install ok installed\nArchitecture: amd64\nVersion: 0.2.0\n\nPackage: halcyon-update-source\nStatus: install ok installed\nArchitecture: all\nVersion: 0.2.0\n\nPackage: halcyon-archive-keyring\nStatus: install ok installed\nArchitecture: all\nVersion: 0.2.0\n\n");
        f.records();
        symlink(f.path(EXE), f.path("/running")).unwrap();
        f
    }
    fn path(&self, path: &str) -> PathBuf {
        rooted(&self.root, path)
    }
    fn put(&self, path: &str, bytes: &[u8]) {
        let p = self.path(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
    fn records(&self) {
        for (name, files) in [
            (MAIN, vec![EXE]),
            ("halcyon-update-source", vec![SOURCE, PREFS]),
            ("halcyon-archive-keyring", vec![KEY]),
        ] {
            self.put(
                &format!("/var/lib/dpkg/info/{name}.list"),
                files.join("\n").as_bytes(),
            );
            let hashes = files
                .iter()
                .map(|path| {
                    format!(
                        "{}  {}\n",
                        md5(&self.path(path)).unwrap(),
                        path.trim_start_matches('/')
                    )
                })
                .collect::<String>();
            self.put(
                &format!("/var/lib/dpkg/info/{name}.md5sums"),
                hashes.as_bytes(),
            );
        }
    }
    fn measure(&self) -> Identity {
        measure(
            &self.root,
            &self.path("/running"),
            &self.path("/build/halcyon"),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn actual_files_package_checksum_source_and_other_path_are_independent() {
    let f = Fixture::new();
    assert_eq!(f.measure().channel, UpdateChannel::Apt);
    fs::remove_file(f.path(SOURCE)).unwrap();
    assert_eq!(f.measure().channel, UpdateChannel::ManualDeb);
    f.put(SOURCE, format!("{SOURCE_TEXT}Enabled: no\n").as_bytes());
    assert_eq!(f.measure().channel, UpdateChannel::ManualDeb);
    f.put(SOURCE, SOURCE_TEXT.as_bytes());
    f.put("/build/halcyon", b"fixture app");
    fs::remove_file(f.path("/running")).unwrap();
    symlink(f.path("/build/halcyon"), f.path("/running")).unwrap();
    assert_eq!(f.measure().channel, UpdateChannel::Source);
    assert_eq!(f.measure().installed_version, None);
    assert_eq!(
        measure(
            &f.root,
            &f.path("/running"),
            &f.path("/different-build/halcyon")
        )
        .channel,
        UpdateChannel::Unknown
    );
    // The genuine package and scoped source still exist beside this process.
    assert!(f.path(EXE).is_file());
    assert!(f.path(SOURCE).is_file());
}
#[test]
fn damaged_owner_architecture_package_or_enrollment_cannot_offer_check() {
    for change in [
        "binary",
        "owner",
        "architecture",
        "status",
        "checksum",
        "key-checksum",
        "key",
        "source",
        "preferences",
        "duplicate-source",
        "diversion",
        "symlink",
        "unknown-source",
    ] {
        let f = Fixture::new();
        match change {
            "binary" => f.put(EXE, b"modified executable"),
            "owner" => f.put(
                &format!("/var/lib/dpkg/info/{MAIN}.list"),
                b"/usr/bin/something-else",
            ),
            "architecture" | "status" => {
                let p = f.path("/var/lib/dpkg/status");
                let s = fs::read_to_string(&p).unwrap();
                fs::write(
                    p,
                    if change == "architecture" {
                        s.replace("amd64", "arm64")
                    } else {
                        s.replace("install ok installed", "install ok unpacked")
                    },
                )
                .unwrap();
            }
            "checksum" => {
                fs::remove_file(f.path(&format!("/var/lib/dpkg/info/{MAIN}.md5sums"))).unwrap();
            }
            "key-checksum" => {
                fs::remove_file(f.path("/var/lib/dpkg/info/halcyon-archive-keyring.md5sums"))
                    .unwrap();
            }
            "key" => f.put(KEY, b"wrong key"),
            "source" => f.put(SOURCE, SOURCE_TEXT.replace("https:", "http:").as_bytes()),
            "unknown-source" => f.put(SOURCE, b"URIs: https://unknown.invalid\n"),
            "preferences" => f.put(PREFS, PREFS_TEXT.replace("-1", "1001").as_bytes()),
            "duplicate-source" => f.put(
                "/etc/apt/sources.list.d/other.sources",
                SOURCE_TEXT.as_bytes(),
            ),
            "diversion" => f.put(
                "/var/lib/dpkg/diversions",
                format!("{EXE}\n/usr/bin/other\nlocal\n").as_bytes(),
            ),
            "symlink" => {
                f.put("/another-app", b"fixture app");
                fs::remove_file(f.path(EXE)).unwrap();
                symlink(f.path("/another-app"), f.path(EXE)).unwrap();
            }
            _ => unreachable!(),
        }
        let result = f.measure();
        assert!(!result.can_check(), "{change}: {result:?}");
        assert_eq!(result.channel, UpdateChannel::Unknown, "{change}");
    }
}
#[test]
fn procfs_running_inode_survives_real_replacement_and_deletion() {
    struct Process(std::process::Child);
    impl Drop for Process {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new();
    fs::copy("/usr/bin/sleep", f.path(EXE)).unwrap();
    fs::set_permissions(f.path(EXE), fs::Permissions::from_mode(0o755)).unwrap();
    f.records();
    let process = Process(Command::new(f.path(EXE)).arg("30").spawn().unwrap());
    let proc_exe = PathBuf::from(format!("/proc/{}/exe", process.0.id()));
    assert_eq!(
        measure(&f.root, &proc_exe, &f.path("/build/halcyon")).channel,
        UpdateChannel::Apt
    );
    fs::copy("/usr/bin/true", f.path("/replacement")).unwrap();
    fs::rename(f.path("/replacement"), f.path(EXE)).unwrap();
    f.records();
    let changed = measure(&f.root, &proc_exe, &f.path("/build/halcyon"));
    assert_eq!(changed.channel, UpdateChannel::Unknown);
    assert_eq!(changed.installed_version.as_deref(), Some("0.2.0"));
    assert!(changed.reason.contains("replaced or removed"));
    fs::remove_file(f.path(EXE)).unwrap();
    let deleted = measure(&f.root, &proc_exe, &f.path("/build/halcyon"));
    assert_eq!(deleted.channel, UpdateChannel::Unknown);
    assert!(deleted.reason.contains("replaced or removed"));
}
#[test]
fn checks_bind_enrollment_bytes_and_ignore_unrelated_multiarch_packages() {
    let f = Fixture::new();
    let first = f.measure();
    let p = f.path("/var/lib/dpkg/status");
    let mut text = fs::read_to_string(&p).unwrap();
    for arch in ["amd64", "i386"] {
        text.push_str(&format!(
            "Package: libc6\nStatus: install ok installed\nArchitecture: {arch}\nVersion: 1.0\n\n"
        ));
    }
    fs::write(p, text).unwrap();
    assert!(first.same_installation(&f.measure()));
    f.put(KEY, &[0x98; 256]);
    f.records();
    assert!(!first.same_installation(&f.measure()));
}
#[test]
fn store_desktop_id_respects_xdg_shadowing_hidden_and_missing_entries() {
    let f = Fixture::new();
    let system = f.path("/system/applications");
    let user = f.path("/user/applications");
    fs::create_dir_all(&system).unwrap();
    fs::create_dir_all(&user).unwrap();
    let dirs = vec![user.clone(), system.clone()];
    let launcher = f.path("/tools/gio");
    let program = f.path("/tools/cosmic-store");
    f.put("/tools/gio", b"fixture");
    f.put("/tools/cosmic-store", b"fixture");
    for path in [&launcher, &program] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    assert!(store_entry(&dirs, &launcher, &program).is_none());
    fs::write(
        system.join(STORE),
        "[Desktop Entry]\nType=Application\nExec=cosmic-store %u\nTerminal=false\n",
    )
    .unwrap();
    assert_eq!(
        store_entry(&dirs, &launcher, &program),
        Some(system.join(STORE))
    );
    fs::remove_file(&program).unwrap();
    assert!(store_entry(&dirs, &launcher, &program).is_none());
    f.put("/tools/cosmic-store", b"fixture");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    for text in [
        "[Desktop Entry]\nType=Application\nExec=cosmic-store %u\nHidden=true\n",
        "[Desktop Entry]\nType=Application\nExec=cosmic-store updates\n",
        "[Desktop Entry]\nType=Application\nExec=other-store\n",
        "[Desktop Entry]\nType=Application\nExec=cosmic-store %u\nDBusActivatable=true\n",
        "[Desktop Entry]\nType=Application\nExec=cosmic-store %u\n[Desktop Entry]\nType=Application\nExec=other-store\n",
        "broken",
    ] {
        fs::write(user.join(STORE), text).unwrap();
        assert!(store_entry(&dirs, &launcher, &program).is_none());
    }
}

#[test]
fn ignored_removal_backups_do_not_disable_reenrollment_but_active_conflicts_do() {
    let f = Fixture::new();
    f.put(
        "/etc/apt/sources.list.d/halcyon.sources.removed.fixture",
        SOURCE_TEXT.as_bytes(),
    );
    assert_eq!(f.measure().channel, UpdateChannel::Apt);
    fs::remove_file(f.path(SOURCE)).unwrap();
    assert_eq!(f.measure().channel, UpdateChannel::ManualDeb);
    f.put(
        "/etc/apt/sources.list.d/another.sources",
        SOURCE_TEXT.as_bytes(),
    );
    assert_eq!(f.measure().channel, UpdateChannel::Unknown);
}
