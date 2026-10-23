// Included in settings::tests to reuse its isolated child and native driver.
struct LockedNativeJob {
    root: PathBuf,
    worker: Option<std::thread::JoinHandle<Vec<Message>>>,
}
impl LockedNativeJob {
    fn arm(store: &crate::desktop::DesktopStore) {
        let root = store.fixture_root().unwrap();
        let config_lock = format!("{}.lock", Config::path().unwrap().display());
        let desktop_lock = store.state_dir().join("desktop.lock");
        // This helper only pauses discovery when BOTH actual transaction locks
        // are already held by its caller. No sleeping-before-start substitute.
        std::fs::write(
            root.join("slow-islands-probe.py"),
            format!(
                r#"
import fcntl, pathlib, time
root=pathlib.Path({root:?})
held=[]
for name in [{config:?}, {desktop:?}]:
    with open(name, 'a+b') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held.append(False)
            fcntl.flock(lock, fcntl.LOCK_UN)
        except BlockingIOError:
            held.append(True)
if all(held):
    (root/'worker-locked').write_text('config+desktop')
    deadline=time.monotonic()+2.0
    while not (root/'worker-release').exists() and time.monotonic()<deadline:
        time.sleep(0.005)
    if not (root/'worker-release').exists():
        (root/'worker-timeout').write_text('watchdog released blocked event handler')
print((root/'islands-installation.json').read_text())
"#,
                root = root.to_str().unwrap(),
                config = config_lock,
                desktop = desktop_lock.to_str().unwrap()
            ),
        )
        .unwrap();
    }
    fn start(store: &crate::desktop::DesktopStore, task: Task<Message>) -> Self {
        use cosmic::iced::futures::StreamExt;
        let worker = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap()
                .block_on(async {
                    let mut messages = Vec::new();
                    if let Some(mut stream) = cosmic::iced::runtime::task::into_stream(task) {
                        while let Some(action) = stream.next().await {
                            if let cosmic::iced::runtime::Action::Output(cosmic::Action::App(
                                message,
                            )) = action
                            {
                                messages.push(message);
                            }
                        }
                    }
                    messages
                })
        });
        let job = Self {
            root: store.fixture_root().unwrap().into(),
            worker: Some(worker),
        };
        let started = Instant::now();
        while !job.root.join("worker-locked").exists() {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "worker must reach authoritative discovery under both locks"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        // Independently prove the marker describes held locks, not a helper
        // that ran before the transaction. LOCK_NB never stalls the test thread.
        use std::os::fd::AsRawFd;
        for path in [
            PathBuf::from(format!("{}.lock", Config::path().unwrap().display())),
            store.state_dir().join("desktop.lock"),
        ] {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .unwrap();
            assert_ne!(
                unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
                0,
                "transaction lock must already be held"
            );
            assert_eq!(
                std::io::Error::last_os_error().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
        job
    }
    fn release(mut self) -> Vec<Message> {
        std::fs::write(self.root.join("worker-release"), b"release").unwrap();
        self.worker.take().unwrap().join().unwrap()
    }
}
impl Drop for LockedNativeJob {
    fn drop(&mut self) {
        let _ = std::fs::write(self.root.join("worker-release"), b"release");
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn queue_pointer(
    app: &mut Settings,
    native: &mut crate::native_test::Native,
    id: &str,
) -> Task<Message> {
    use cosmic::iced::{Event, mouse};
    let bounds = native.bounds(app.settings_view(), id);
    assert!(
        bounds.y >= 0. && bounds.y + bounds.height <= native.size.height,
        "{id} visible: {bounds:?}"
    );
    let mut tasks = Vec::new();
    let mut emitted = false;
    for event in [
        mouse::Event::CursorMoved {
            position: bounds.center(),
        },
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        for message in native
            .event(app.settings_view(), Event::Mouse(event), bounds.center())
            .0
        {
            emitted = true;
            tasks.push(app.update(message));
        }
    }
    assert!(emitted, "{id} must emit a pointer action");
    Task::batch(tasks)
}
fn queue_fixture(
    page: Page,
) -> (
    crate::desktop::DesktopStore,
    Settings,
    crate::native_test::Native,
) {
    let store = crate::desktop::DesktopStore::from_env().unwrap();
    crate::desktop::islands::fixture_installed(store.fixture_root().unwrap());
    let (mut app, initial) = Settings::init(Core::default(), page);
    settle_settings(&mut app, initial);
    let size = Size::new(1024., 900.);
    send_settings(&mut app, Message::Resized(size));
    (store, app, crate::native_test::Native::new(size))
}
#[test]
fn task2r2_style_toggle_defers_while_helper_holds_transaction_locks() {
    if desktop_test_child(
        "task2r2_style_toggle_defers_while_helper_holds_transaction_locks",
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    let (store, mut app, mut native) = queue_fixture(Page::Themes);
    send_settings(
        &mut app,
        Message::Profiles(profiles::Message::Select(profiles::Selection::Preset(
            "theme-soft",
        ))),
    );
    send_settings(&mut app, Message::StyleBrowser(true));
    native.size = Size::new(1024., 1800.);
    send_settings(&mut app, Message::Resized(native.size));
    let _ = native.render(app.settings_view(), &controls_theme(&app.config));
    let before = Config::load().appearance.style_bars;
    LockedNativeJob::arm(&store);
    let task = queue_pointer(&mut app, &mut native, "profiles-confirm");
    let job = LockedNativeJob::start(&store, task);
    let started = Instant::now();
    let toggle = queue_pointer(&mut app, &mut native, "theme-style-bars");
    let elapsed = started.elapsed();
    // Release before asserting so RED has a deterministic bounded shutdown.
    // Other save()/Config writers absent from the old classifier must take
    // the same admission path. Window resize/preview must stay read-only.
    let queued = app.native_deferred.len();
    let _ = app.update(Message::Resized(native.size));
    let _ = app.update(Message::Preview);
    assert_eq!(app.native_deferred.len(), queued);
    let raw_before = std::fs::read(Config::path().unwrap()).unwrap();
    let mut other_tasks = Vec::new();
    for message in [
        Message::SetupDismissed(true),
        Message::Desktop(false),
        Message::WidgetFont(FontTarget::Text, Some("Fira Sans".into())),
    ] {
        let started = Instant::now();
        other_tasks.push(app.update(message));
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "equivalent save path must not enter worker-held locks"
        );
    }
    assert_eq!(std::fs::read(Config::path().unwrap()).unwrap(), raw_before);
    assert_eq!(app.native_deferred.len(), queued + 3);
    let timed_out = job.root.join("worker-timeout").exists();
    let still_before = Config::load().appearance.style_bars == before;
    let messages = job.release();
    assert!(
        elapsed < Duration::from_millis(250),
        "style pointer waited on active worker locks: {elapsed:?}"
    );
    assert!(
        !timed_out && still_before,
        "toggle must wait in the queue, not on a lock"
    );
    settle_settings(
        &mut app,
        Task::batch(std::iter::once(toggle).chain(other_tasks)),
    );
    for message in messages {
        send_settings(&mut app, message);
    }
    assert_eq!(Config::load().appearance.style_bars, !before);
    let raw = raw_preferences();
    assert_eq!(raw["setup_dismissed"], true);
    assert_eq!(raw["widgets_visible"], false);
    assert_eq!(raw["widget_font"], "Fira Sans");
    println!(
        "Style pointer dispatch {elapsed:?}; both transaction locks held; deferred toggle persisted"
    );
}
#[test]
fn task2r2_rendered_undo_a_cannot_reverse_worker_b() {
    if desktop_test_child(
        "task2r2_rendered_undo_a_cannot_reverse_worker_b",
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    let (store, mut app, mut native) = queue_fixture(Page::Appearance);
    native_click(&mut app, &mut native, "material-frosted");
    let a = store.latest_change().unwrap().unwrap().id;
    assert_eq!(app.last_desktop_change.as_ref().unwrap().2, a);
    LockedNativeJob::arm(&store);
    let b = queue_pointer(&mut app, &mut native, "material-clear");
    let job = LockedNativeJob::start(&store, b);
    let undo = queue_pointer(&mut app, &mut native, "settings-undo");
    assert_eq!(
        app.last_desktop_change.as_ref().unwrap().2,
        a,
        "click was Undo A"
    );
    let messages = job.release();
    let b = store.latest_change().unwrap().unwrap().id;
    assert_ne!(a, b);
    settle_settings(&mut app, undo);
    for message in messages {
        send_settings(&mut app, message);
    }
    assert_eq!(
        store.latest_change().unwrap().unwrap().id,
        b,
        "queued Undo A must not undo unseen B"
    );
    assert_eq!(
        Config::load().materials.shared.unwrap().kind,
        crate::materials::Kind::Clear
    );
    assert!(
        app.error && app.status.contains("changed"),
        "stale Undo must visibly refuse: {}",
        app.status
    );
    println!("Rendered Undo A={a}; committed B={b}; B retained; stale request refused");
}
#[test]
fn task2r2_rendered_conflict_resolution_retains_its_review() {
    if desktop_test_child(
        "task2r2_rendered_conflict_resolution_retains_its_review",
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    for button in ["undo-reviewed", "undo-keep-current"] {
        let (store, mut app, mut native) = queue_fixture(Page::Appearance);
        let root = store.fixture_root().unwrap();
        for name in [
            "slow-islands-probe.py",
            "worker-locked",
            "worker-release",
            "worker-timeout",
        ] {
            let _ = std::fs::remove_file(root.join(name));
        }
        native_click(&mut app, &mut native, "material-frosted");
        let a = store.latest_change().unwrap().unwrap().id;
        // Outside writer creates A's partial Undo review.
        let mut raw = raw_preferences();
        raw["materials"]["shared"] = serde_json::to_value(crate::materials::Finish::new(
            crate::materials::Kind::Smoked,
        ))
        .unwrap();
        std::fs::write(Config::path().unwrap(), serde_json::to_vec(&raw).unwrap()).unwrap();
        app.undo_review = Some(store.review_undo(&a).unwrap());
        assert!(!app.undo_review.as_ref().unwrap().conflicts.is_empty());
        LockedNativeJob::arm(&store);
        let b = queue_pointer(&mut app, &mut native, "material-clear");
        let job = LockedNativeJob::start(&store, b);
        let resolution = queue_pointer(&mut app, &mut native, button);
        let messages = job.release();
        let b = store.latest_change().unwrap().unwrap().id;
        assert_ne!(a, b);
        // A later review replaces UI state before queued consumption. The
        // pointer's captured review must not be read back from this live slot.
        app.undo_review = Some(store.review_undo(&b).unwrap());
        settle_settings(&mut app, resolution);
        for message in messages {
            send_settings(&mut app, message);
        }
        assert_eq!(
            store.latest_change().unwrap().unwrap().id,
            b,
            "{button} must not resolve a replacement review"
        );
        assert_eq!(
            Config::load().materials.shared.unwrap().kind,
            crate::materials::Kind::Clear
        );
        assert!(app.error, "stale conflict resolution must refuse");
        println!("{button}: captured A={a}, replacement review B={b}; B retained");
    }
}

#[test]
fn task2r3_rendered_undo_refuses_unfinished_keyboard_gesture() {
    if desktop_test_child(
        "task2r3_rendered_undo_refuses_unfinished_keyboard_gesture",
        DesktopEnv {
            app_config: true,
            fixture: true,
            ..Default::default()
        },
    ) {
        return;
    }
    let (store, mut app, mut native) = queue_fixture(Page::Appearance);
    native.size = Size::new(1280., 1800.);
    send_settings(&mut app, Message::Resized(native.size));
    send_settings(&mut app, Message::ToggleAdvanced(Page::Appearance));
    let before = Config::load();
    send_settings(&mut app, Message::ClockDate(!before.clock.show_date));
    let a = store.latest_change().unwrap().unwrap().id;
    assert!(!app.native_busy);
    let bounds = native.text_bounds(app.settings_view(), "Corners");
    let point = cosmic::iced::Point::new(bounds.x + 100., bounds.y + bounds.height + 12.);
    assert!(point.y < native.size.height, "keyboard slider is visible");
    let messages = r2_native_event(
        &mut app,
        &mut native,
        repair_slider_event(true, false),
        point,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::AppearanceValue(AppearanceControl::Corners, _))),
        "actual Corners slider receives held arrow: {messages:?}"
    );
    assert!(app.appearance_editing && !app.appearance_pending.is_empty());
    let draft = app.appearance_draft.corner_radius;
    assert_ne!(draft, before.appearance.corner_radius);
    assert_eq!(
        Config::load().appearance.corner_radius,
        before.appearance.corner_radius
    );
    assert_eq!(store.latest_change().unwrap().unwrap().id, a);

    let request = queue_pointer(&mut app, &mut native, "settings-undo");
    settle_settings(&mut app, request);
    assert_eq!(
        Config::load().clock.show_date,
        !before.clock.show_date,
        "unfinished gesture must not let historical Undo reverse older Clock A"
    );
    assert_eq!(store.latest_change().unwrap().unwrap().id, a);
    assert!(
        app.error && app.status.contains("Finish the current edit"),
        "explicit refusal: {}",
        app.status
    );
    assert!(app.appearance_editing && !app.appearance_pending.is_empty());
    assert_eq!(
        Config::load().appearance.corner_radius,
        before.appearance.corner_radius
    );

    // Refusal did not claim to undo B. Releasing completes B normally; only
    // the newly displayed Undo B can reverse it, leaving disjoint Clock A.
    let footer = native.bounds(app.settings_view(), "settings-undo").center();
    let messages = r2_native_event(
        &mut app,
        &mut native,
        repair_slider_event(false, false),
        footer,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::SaveAppearance))
    );
    assert!(!app.appearance_editing && app.appearance_pending.is_empty());
    assert_eq!(Config::load().appearance.corner_radius, draft);
    assert_eq!(Config::load().clock.show_date, !before.clock.show_date);
    let b = store.latest_change().unwrap().unwrap().id;
    assert_ne!(a, b);
    let request = queue_pointer(&mut app, &mut native, "settings-undo");
    settle_settings(&mut app, request);
    assert!(!app.error, "completed B Undo: {}", app.status);
    assert_eq!(
        Config::load().appearance.corner_radius,
        before.appearance.corner_radius
    );
    assert_eq!(Config::load().clock.show_date, !before.clock.show_date);
    let undone = raw_preferences();
    let history = store.latest_change().unwrap().map(|c| c.id);
    let messages = r2_native_event(
        &mut app,
        &mut native,
        repair_slider_event(false, false),
        footer,
    );
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, Message::SaveAppearance))
    );
    assert_eq!(
        raw_preferences(),
        undone,
        "late release must not reapply undone B"
    );
    assert_eq!(store.latest_change().unwrap().map(|c| c.id), history);
    println!(
        "Actual held Corners gesture: Undo A={a} refused; release committed B={b}; rendered Undo B preserved Clock; late release saved nothing"
    );
}
