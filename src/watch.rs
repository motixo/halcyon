//! Watch the containing directories so atomic config/command replacements are seen.
//! Only changed contents reach the UI; the fallback also recovers missed events.
use cosmic::iced::{Subscription, futures::SinkExt, stream};
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;

pub fn files(paths: Vec<PathBuf>) -> Subscription<()> {
    Subscription::run_with(paths, |paths| changes(paths.clone()))
}

/// Observe settings owned by the desktop without creating namespaces or
/// files. If a namespace is not present yet, the fallback notices it once
/// created; the desktop remains the sole owner of its directories.
pub fn read_only_files(paths: Vec<PathBuf>) -> Subscription<()> {
    Subscription::run_with(paths, |paths| changes_with(paths.clone(), false))
}

fn changes(paths: Vec<PathBuf>) -> impl cosmic::iced::futures::Stream<Item = ()> {
    changes_with(paths, true)
}

fn changes_with(
    paths: Vec<PathBuf>,
    create_parents: bool,
) -> impl cosmic::iced::futures::Stream<Item = ()> {
    stream::channel(1, async move |mut output| {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let watched = paths.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let relevant = match event {
                    Ok(event) => {
                        matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                        ) && event.paths.iter().any(|path| watched.contains(path))
                    }
                    Err(_) => true,
                };
                if relevant {
                    let _ = sender.try_send(());
                }
            })
            .ok();
        if let Some(watcher) = &mut watcher {
            let mut directories = Vec::new();
            for path in &paths {
                if let Some(parent) = path.parent() {
                    if !directories.contains(&parent) {
                        if create_parents {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
                        directories.push(parent);
                    }
                }
            }
        }
        let mut previous = None;
        let mut fallback = tokio::time::interval(Duration::from_secs(30));
        fallback.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                signal = receiver.recv(), if watcher.is_some() => {
                    if signal.is_none() { watcher = None; }
                    // Coalesce the write/rename pair, and avoid reading a partial write.
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    while receiver.try_recv().is_ok() {}
                }
                _ = fallback.tick() => {}
            }
            let current: Vec<_> = paths.iter().map(|path| std::fs::read(path).ok()).collect();
            if previous.as_ref() != Some(&current) {
                previous = Some(current);
                if output.send(()).await.is_err() {
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::iced::futures::{StreamExt, pin_mut};

    #[tokio::test]
    async fn read_only_watch_does_not_create_absent_desktop_namespaces() {
        let root = std::env::temp_dir().join(format!(
            "desktop-cards-read-only-watch-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let events = changes_with(vec![root.join("namespace/v2/accent")], false);
        pin_mut!(events);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), events.next())
                .await
                .unwrap(),
            Some(())
        );
        assert!(!root.exists());
    }

    #[tokio::test]
    async fn atomic_replacement_wakes_ui_but_reads_and_identical_writes_do_not() {
        let directory =
            std::env::temp_dir().join(format!("desktop-cards-watch-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.json");
        std::fs::write(&path, "old").unwrap();
        let events = changes(vec![path.clone()]);
        pin_mut!(events);
        let deadline = Duration::from_secs(2);
        assert_eq!(
            tokio::time::timeout(deadline, events.next()).await.unwrap(),
            Some(())
        );
        let replacement = directory.join("replacement.tmp");
        std::fs::write(&replacement, "new").unwrap();
        std::fs::rename(replacement, &path).unwrap();
        assert_eq!(
            tokio::time::timeout(deadline, events.next()).await.unwrap(),
            Some(())
        );
        std::fs::read(&path).unwrap();
        std::fs::write(&path, "new").unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(150), events.next())
                .await
                .is_err()
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            tokio::time::timeout(deadline, events.next()).await.unwrap(),
            Some(())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
