//! Real client proxies on an isolated socketpair; no host Wayland connection.
use super::*;
use cosmic::cctk::sctk::reexports::client::{
    Connection, delegate_noop,
    protocol::{wl_output, wl_registry},
};
use std::os::unix::net::UnixStream;

pub struct Client;
delegate_noop!(Client: ignore wl_registry::WlRegistry);
delegate_noop!(Client: ignore wl_output::WlOutput);
pub struct Wire {
    connection: Connection,
    _server: UnixStream,
    queue: cosmic::cctk::sctk::reexports::client::EventQueue<Client>,
    registry: wl_registry::WlRegistry,
}
impl Wire {
    pub fn new() -> Self {
        let (client, server) = UnixStream::pair().unwrap();
        let connection = Connection::from_socket(client).unwrap();
        let queue = connection.new_event_queue::<Client>();
        let registry = connection.display().get_registry(&queue.handle(), ());
        Self {
            connection,
            _server: server,
            queue,
            registry,
        }
    }
    pub fn output(&self, global: u32) -> WlOutput {
        let _ = &self.connection;
        self.registry.bind(global, 4, &self.queue.handle(), ())
    }
}
pub fn info(name: Option<&str>, size: (i32, i32)) -> OutputInfo {
    let mut info = cosmic::cctk::sctk::output::OutputData::new(1).with_output_info(Clone::clone);
    info.model = "Identical model".into();
    info.make = "Example".into();
    info.logical_size = Some(size);
    info.logical_position = Some((0, 0));
    info.name = name.map(str::to_owned);
    info
}

/// The real application reads only this child process's fixture preferences.
pub fn isolated(name: &str) -> bool {
    if std::env::var("HALCYON_MONITOR_TEST").as_deref() == Ok(name) {
        return false;
    }
    let root = std::env::temp_dir().join(format!(
        "halcyon-monitor-{}-{}",
        std::process::id(),
        name.replace(':', "_")
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("config.json"), "{}").unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("HALCYON_MONITOR_TEST", name)
        .env("DESKTOP_CARDS_CONFIG", root.join("config.json"))
        .env("DESKTOP_CARDS_DESKTOP_ROOT", &root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .status()
        .unwrap();
    assert!(status.success());
    true
}
