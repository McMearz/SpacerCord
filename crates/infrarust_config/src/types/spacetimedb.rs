//! SpacetimeDB connection configuration.
//!
//! SpacerCord runs SpacetimeDB as a child process by default so operators get a
//! one-click cycle: install if missing, kill the port owner, spawn the server,
//! poll readiness, publish the bundled barebones module on first boot, then
//! connect via the SDK.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for the SpacetimeDB integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpacetimeConfig {
    /// Master switch. When false the integration is fully off; the runtime is
    /// not spawned and the no-op service is installed.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Public URI used by the SDK to connect (e.g. `http://127.0.0.1:3000`).
    /// Should match `listen` for the embedded-server case.
    #[serde(default = "default_uri")]
    pub uri: String,

    /// Address the spawned `spacetimedb start` child listens on.
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,

    /// Database/module name to publish + connect to.
    #[serde(default = "default_db_name")]
    pub db_name: String,

    /// Path to the `spacetimedb` CLI binary. Resolved through PATH when
    /// relative; well-known install paths are also probed.
    #[serde(default = "default_binary")]
    pub binary: PathBuf,

    /// If the CLI binary is missing, run the official installer
    /// (`iwr https://windows.spacetimedb.com -useb | iex` on Windows,
    /// `curl -sSf https://install.spacetimedb.com | sh` elsewhere).
    #[serde(default = "default_true")]
    pub auto_install: bool,

    /// Working data directory for the spawned server.
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Path to the SpacetimeDB module project. The proxy publishes this once
    /// on first boot if `db_name` doesn't exist on the host yet, then never
    /// touches it again — manual republishes go through the admin panel.
    #[serde(default = "default_module_path")]
    pub module_path: PathBuf,

    /// Maximum time to wait for the spawned server to become ready
    /// (responds to `/v1/ping`) before giving up.
    #[serde(default = "default_ready_timeout", with = "humantime_serde")]
    pub ready_timeout: Duration,

    /// Time the proxy gives the child to exit gracefully on shutdown/restart
    /// before SIGKILL / `TerminateProcess`.
    #[serde(default = "default_stop_timeout", with = "humantime_serde")]
    pub stop_timeout: Duration,
}

fn default_true() -> bool {
    true
}

fn default_uri() -> String {
    "http://127.0.0.1:3000".to_string()
}

fn default_listen() -> SocketAddr {
    "127.0.0.1:3000".parse().expect("valid default listen addr")
}

fn default_db_name() -> String {
    "spacer-cord".to_string()
}

fn default_binary() -> PathBuf {
    PathBuf::from("spacetime")
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./stdb-data")
}

fn default_module_path() -> PathBuf {
    PathBuf::from("./stdb-module")
}

fn default_ready_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_stop_timeout() -> Duration {
    Duration::from_secs(8)
}

impl Default for SpacetimeConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            uri: default_uri(),
            listen: default_listen(),
            db_name: default_db_name(),
            binary: default_binary(),
            auto_install: default_true(),
            data_dir: default_data_dir(),
            module_path: default_module_path(),
            ready_timeout: default_ready_timeout(),
            stop_timeout: default_stop_timeout(),
        }
    }
}
