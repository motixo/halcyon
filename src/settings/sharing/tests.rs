use super::*;

#[test]
#[ignore = "Owned private scene worker subprocess"]
fn worker_entry() {
    super::main();
}

#[test]
fn fb13_public_scene_omits_paths_provenance_and_runtime_assignments() {
    let mut profile = profiles::preset("paper-studio").unwrap().profile;
    profile
        .wallpaper
        .as_mut()
        .unwrap()
        .background
        .as_mut()
        .unwrap()
        .source = Some(SourceLook::Image("/private/secret.jpg".into()));
    let scene = Scene::new(&profile).unwrap();
    let bytes = serde_json::to_string(&scene).unwrap();
    assert!(!bytes.contains("secret"));
    assert!(scene.omitted_picture);
    assert!(scene.background().is_none());
    let config = scene.config().unwrap();
    assert!(config.location.is_none());
    let snapshot = crate::layout_state::public_preview(&config);
    assert_eq!(snapshot.screen, [1920., 1080.]);
}

#[test]
fn fb13_public_scene_worker_renders_both_faces_and_rejects_cancelled_work() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    for id in ["tidal", "paper-studio", "spine", "ridge"] {
        let scene = Scene::new(&profiles::preset(id).unwrap().profile).unwrap();
        let (sender, cancel) = watch::channel(false);
        let png = runtime
            .block_on(
                Job {
                    id: 1,
                    scene: scene.clone(),
                    cancel,
                }
                .run(),
            )
            .unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1280, 720));
        if let Ok(folder) = std::env::var("HALCYON_SHARE_EVIDENCE") {
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(std::path::Path::new(&folder).join(format!("{id}.png")), png).unwrap();
        }
        sender.send_replace(true);
        assert!(
            runtime
                .block_on(
                    Job {
                        id: 2,
                        scene,
                        cancel: sender.subscribe()
                    }
                    .run()
                )
                .is_err()
        );
    }
}
