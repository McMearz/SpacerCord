//! Managed SpacetimeDB lifecycle.
//!
//! Owns the full one-click cycle: detect or auto-install the `spacetime` CLI,
//! free the listen port, spawn `spacetime start` as a child, capture its
//! stdout/stderr into a shared ring buffer, poll readiness, publish the
//! barebones module on first boot, then connect via the SDK.
//!
//! The runtime is held by the proxy core (`services.spacetimedb`) and exposed
//! to the admin panel for status queries, restarts, manual publishes, schema
//! lookups, and live log/event streams.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use infrarust_config::types::SpacetimeConfig;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, broadcast};
use tokio::time;

use crate::log_ring::{LogSource, StdbLogBroadcast};
use crate::{NotificationOp, SpacetimeHandle, SpacetimeNotification};

/// Server-process lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerState {
    /// Disabled in config; nothing was started.
    Disabled,
    /// Initial state before first start completes.
    Pending,
    /// Auto-install running.
    Installing,
    /// Spawn / port-free / ready-poll in progress.
    Starting,
    /// Process is running and `/v1/ping` returned 200 within the timeout.
    Running,
    /// Restart is in progress.
    Restarting,
    /// Lifecycle finished with an error.
    Failed,
    /// Proxy is shutting down.
    Stopped,
}

/// SDK connection state, separate from the server-process state because the
/// child can be running without the SDK being connected (e.g. mid-restart).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

/// Snapshot returned to the admin panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub enabled: bool,
    pub server_state: ServerState,
    pub connection_state: ConnectionState,
    pub uri: String,
    pub listen: String,
    pub db_name: String,
    pub binary_resolved: Option<String>,
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub module_published: bool,
}

/// Row-change event for the panel's live feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowEvent {
    pub timestamp: DateTime<Utc>,
    pub table_name: String,
    pub operation: RowOp,
    pub data_len: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowOp {
    Insert,
    Update,
    Delete,
}

/// Mutable state guarded by the runtime mutex.
struct Inner {
    server_state: ServerState,
    connection_state: ConnectionState,
    pid: Option<u32>,
    started_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    binary_resolved: Option<PathBuf>,
    module_published: bool,
    /// Owned child process (kept alive for kill-on-drop).
    child: Option<Child>,
    /// Live SDK handle. None during restart or when connect failed.
    handle: Option<SpacetimeHandle>,
}

/// External notify callback type — typically the EventBus bridge from
/// `infrarust-core` that fires `SpacetimeRowEvent` on the proxy bus.
pub type ExternalNotify = Arc<dyn Fn(SpacetimeNotification) + Send + Sync>;

/// Managed SpacetimeDB lifecycle.
pub struct SpacetimeRuntime {
    cfg: SpacetimeConfig,
    inner: Mutex<Inner>,
    pub logs: Arc<StdbLogBroadcast>,
    pub events: broadcast::Sender<RowEvent>,
    /// External callback invoked alongside the panel's row-event broadcast on
    /// every SDK row update. Stored on the runtime so it can be re-bound to
    /// each fresh `SpacetimeHandle` on restart.
    external_notify: Mutex<Option<ExternalNotify>>,
}

impl SpacetimeRuntime {
    /// Create a runtime in the pending state. Call [`SpacetimeRuntime::start`]
    /// to drive it through the full one-click cycle.
    pub fn new(cfg: SpacetimeConfig) -> Arc<Self> {
        let (events_tx, _) = broadcast::channel::<RowEvent>(256);
        let initial_state = if cfg.enabled {
            ServerState::Pending
        } else {
            ServerState::Disabled
        };
        Arc::new(Self {
            cfg,
            inner: Mutex::new(Inner {
                server_state: initial_state,
                connection_state: ConnectionState::Disconnected,
                pid: None,
                started_at: None,
                last_error: None,
                binary_resolved: None,
                module_published: false,
                child: None,
                handle: None,
            }),
            logs: Arc::new(StdbLogBroadcast::new(512, 1000)),
            events: events_tx,
            external_notify: Mutex::new(None),
        })
    }

    /// Register the external row-change callback. Call before `start()` so the
    /// first SDK connection picks it up; subsequent restarts will reuse it.
    pub async fn set_external_notify(&self, cb: ExternalNotify) {
        *self.external_notify.lock().await = Some(cb);
    }

    /// True if the integration is enabled in config.
    pub fn is_enabled(&self) -> bool {
        self.cfg.enabled
    }

    pub fn config(&self) -> &SpacetimeConfig {
        &self.cfg
    }

    pub async fn handle(&self) -> Option<SpacetimeHandle> {
        self.inner.lock().await.handle.clone()
    }

    pub async fn status(&self) -> RuntimeStatus {
        let g = self.inner.lock().await;
        RuntimeStatus {
            enabled: self.cfg.enabled,
            server_state: g.server_state.clone(),
            connection_state: g.connection_state.clone(),
            uri: self.cfg.uri.clone(),
            listen: self.cfg.listen.to_string(),
            db_name: self.cfg.db_name.clone(),
            binary_resolved: g.binary_resolved.as_ref().map(|p| p.display().to_string()),
            pid: g.pid,
            started_at: g.started_at,
            last_error: g.last_error.clone(),
            module_published: g.module_published,
        }
    }

    /// Drive the full one-click cycle. Always returns; failures are recorded
    /// in [`RuntimeStatus`] rather than propagated, so the proxy keeps running.
    pub async fn start(self: &Arc<Self>) {
        if !self.cfg.enabled {
            self.set_server_state(ServerState::Disabled, None).await;
            tracing::info!("SpacetimeDB integration disabled in config");
            return;
        }

        if let Err(e) = self.start_inner().await {
            let msg = format!("{e:#}");
            self.logs.push(LogSource::Runtime, format!("[boot] failed: {msg}"));
            tracing::warn!(error = %msg, "SpacetimeDB runtime failed to start");
            self.set_server_state(ServerState::Failed, Some(msg)).await;
        }
    }

    async fn start_inner(self: &Arc<Self>) -> Result<()> {
        // 1. Resolve the CLI binary, auto-installing if missing + permitted.
        let binary = self.resolve_binary().await?;
        {
            let mut g = self.inner.lock().await;
            g.binary_resolved = Some(binary.clone());
        }

        // 2. Free the listen port (kill whatever's there).
        self.set_server_state(ServerState::Starting, None).await;
        if let Err(e) = self.free_port(self.cfg.listen.port()).await {
            self.logs
                .push(LogSource::Runtime, format!("[port-free] {e:#}"));
        }

        // 3. Spawn the server.
        self.spawn_server(&binary).await?;

        // 4. Wait for readiness.
        self.wait_for_ready().await?;
        self.set_server_state(ServerState::Running, None).await;

        // 5. First-run publish if the DB doesn't exist on the host yet.
        let already_published = self.module_exists().await.unwrap_or(false);
        if !already_published {
            self.logs.push(
                LogSource::Runtime,
                format!(
                    "[bootstrap] db `{}` not present, publishing barebones module from {}",
                    self.cfg.db_name,
                    self.cfg.module_path.display()
                ),
            );
            if let Err(e) = self.publish(&binary).await {
                self.logs
                    .push(LogSource::Runtime, format!("[bootstrap] publish failed: {e:#}"));
                tracing::warn!(error = %e, "first-run publish failed; continuing without bootstrap");
            } else {
                self.set_module_published(true).await;
            }
        } else {
            self.set_module_published(true).await;
            self.logs.push(
                LogSource::Runtime,
                format!("[bootstrap] db `{}` already published, skipping", self.cfg.db_name),
            );
        }

        // 6. Connect via the SDK.
        self.connect_sdk().await
    }

    /// Restart the full cycle. Returns once the new state has been observed
    /// (Running or Failed).
    pub async fn restart(self: &Arc<Self>) -> Result<()> {
        self.set_server_state(ServerState::Restarting, None).await;
        self.logs.push(LogSource::Runtime, "[restart] requested");

        // Tear down the SDK first so it doesn't see the WS disconnect as a transient blip.
        self.disconnect_sdk().await;

        // Kill the child.
        self.kill_child().await;

        // Re-run the full cycle.
        self.start().await;

        Ok(())
    }

    /// Manual publish trigger (panel "Publish" button). Streams output into
    /// the shared log ring.
    pub async fn publish_now(self: &Arc<Self>) -> Result<()> {
        let binary = {
            let g = self.inner.lock().await;
            g.binary_resolved.clone().ok_or_else(|| anyhow!("CLI binary not resolved yet"))?
        };
        self.publish(&binary).await?;
        self.set_module_published(true).await;
        Ok(())
    }

    /// Subscribe to the row-change event broadcast.
    pub fn subscribe_events(&self) -> broadcast::Receiver<RowEvent> {
        self.events.subscribe()
    }

    /// Build the notify callback the SDK driver invokes for row changes.
    ///
    /// Forwards into both the panel's broadcast channel and an external
    /// callback (the existing event-bus bridge in `infrarust-core`).
    pub fn make_notify_callback(
        self: &Arc<Self>,
        external: Box<dyn Fn(SpacetimeNotification) + Send + Sync>,
    ) -> Box<dyn Fn(SpacetimeNotification) + Send + Sync> {
        let runtime = Arc::clone(self);
        Box::new(move |notif: SpacetimeNotification| {
            // Mirror to the panel broadcast.
            let SpacetimeNotification::RowUpdate {
                ref table_name,
                ref operation,
                ref data,
            } = notif;

            let op = match operation {
                NotificationOp::Insert => RowOp::Insert,
                NotificationOp::Update => RowOp::Update,
                NotificationOp::Delete => RowOp::Delete,
            };
            let _ = runtime.events.send(RowEvent {
                timestamp: Utc::now(),
                table_name: table_name.clone(),
                operation: op,
                data_len: data.len(),
            });
            external(notif);
        })
    }

    /// Cleanly shut the runtime down. Called on proxy exit.
    pub async fn shutdown(self: &Arc<Self>) {
        self.logs.push(LogSource::Runtime, "[shutdown] tearing down");
        self.disconnect_sdk().await;
        self.kill_child().await;
        self.set_server_state(ServerState::Stopped, None).await;
    }

    // ───── Internals ─────

    async fn set_server_state(&self, s: ServerState, last_error: Option<String>) {
        let mut g = self.inner.lock().await;
        g.server_state = s;
        if last_error.is_some() {
            g.last_error = last_error;
        }
    }

    async fn set_connection_state(&self, s: ConnectionState) {
        self.inner.lock().await.connection_state = s;
    }

    async fn set_module_published(&self, v: bool) {
        self.inner.lock().await.module_published = v;
    }

    /// Resolve the `spacetime` CLI: try config path, then PATH, then the
    /// well-known install location. If still missing and `auto_install`,
    /// run the official installer and re-probe.
    async fn resolve_binary(self: &Arc<Self>) -> Result<PathBuf> {
        if let Some(p) = self.probe_binary().await {
            self.logs.push(
                LogSource::Runtime,
                format!("[resolve] using {}", p.display()),
            );
            return Ok(p);
        }

        if !self.cfg.auto_install {
            return Err(anyhow!(
                "`spacetime` CLI not found and auto_install=false; install it manually or set spacetimedb.auto_install = true"
            ));
        }

        self.set_server_state(ServerState::Installing, None).await;
        self.logs.push(
            LogSource::Runtime,
            "[install] CLI not found; running official installer",
        );
        run_installer(&self.logs).await?;

        self.probe_binary()
            .await
            .ok_or_else(|| anyhow!("installer ran but `spacetime` is still not on PATH or in the well-known install location"))
    }

    async fn probe_binary(&self) -> Option<PathBuf> {
        // 1. As configured, possibly relative — just try to invoke --version.
        if try_version(&self.cfg.binary).await.is_some() {
            return Some(self.cfg.binary.clone());
        }
        // 2. Well-known platform install paths.
        for cand in well_known_install_paths() {
            if try_version(&cand).await.is_some() {
                return Some(cand);
            }
        }
        None
    }

    /// Best-effort: terminate whatever process is bound to `port` so we can
    /// take it over. Logs both success and failure but never returns Err for
    /// "nothing was listening".
    async fn free_port(self: &Arc<Self>, port: u16) -> Result<()> {
        // Quick check: try to bind. If it succeeds, the port is already free.
        if let Ok(listener) = tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            drop(listener);
            return Ok(());
        }
        self.logs.push(
            LogSource::Runtime,
            format!("[port-free] port {port} is busy, asking OS to release it"),
        );

        #[cfg(target_os = "windows")]
        let result = {
            let cmd = format!(
                "Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue \
                 | Select-Object -ExpandProperty OwningProcess \
                 | ForEach-Object {{ Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }}"
            );
            run_capture("powershell.exe", &["-NoProfile", "-Command", &cmd], &self.logs, LogSource::Runtime).await
        };

        #[cfg(not(target_os = "windows"))]
        let result = {
            // Try fuser first, fall back to lsof.
            let cmd = format!(
                "fuser -k {port}/tcp 2>/dev/null || (lsof -ti :{port} | xargs -r kill -9)"
            );
            run_capture("sh", &["-c", &cmd], &self.logs, LogSource::Runtime).await
        };

        // Give the OS a moment to release the port.
        time::sleep(Duration::from_millis(300)).await;
        result
    }

    async fn spawn_server(self: &Arc<Self>, binary: &Path) -> Result<()> {
        // Make sure data_dir exists; spacetime start requires it.
        if !self.cfg.data_dir.exists() {
            tokio::fs::create_dir_all(&self.cfg.data_dir)
                .await
                .with_context(|| format!("creating data_dir {}", self.cfg.data_dir.display()))?;
        }

        let listen = self.cfg.listen.to_string();
        let data_dir = self.cfg.data_dir.to_string_lossy().to_string();

        let mut cmd = Command::new(binary);
        cmd.arg("start")
            .arg("--listen-addr")
            .arg(&listen)
            .arg("--data-dir")
            .arg(&data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        self.logs.push(
            LogSource::Runtime,
            format!(
                "[spawn] {} start --listen-addr {} --data-dir {}",
                binary.display(),
                listen,
                data_dir
            ),
        );

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning {}", binary.display()))?;
        let pid = child.id();

        // Pipe stdout / stderr into the log ring on background tasks.
        if let Some(stdout) = child.stdout.take() {
            let logs = Arc::clone(&self.logs);
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    logs.push(LogSource::Server, line);
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            let logs = Arc::clone(&self.logs);
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    logs.push(LogSource::ServerErr, line);
                }
            });
        }

        let mut g = self.inner.lock().await;
        g.child = Some(child);
        g.pid = pid;
        g.started_at = Some(Utc::now());
        g.last_error = None;
        Ok(())
    }

    async fn wait_for_ready(self: &Arc<Self>) -> Result<()> {
        let url = format!("{}/v1/ping", self.cfg.uri.trim_end_matches('/'));
        let deadline = Instant::now() + self.cfg.ready_timeout;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .context("building readiness http client")?;

        self.logs.push(
            LogSource::Runtime,
            format!(
                "[ready] polling {url} (timeout {}s)",
                self.cfg.ready_timeout.as_secs()
            ),
        );

        loop {
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "spacetime server did not become ready within {}s",
                    self.cfg.ready_timeout.as_secs()
                ));
            }
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    self.logs.push(LogSource::Runtime, "[ready] server is up");
                    return Ok(());
                }
                _ => {
                    time::sleep(Duration::from_millis(400)).await;
                }
            }
        }
    }

    /// Best-effort check whether the database name is already published on the
    /// host. Implemented via the HTTP admin route `/v1/database/<name>`.
    async fn module_exists(&self) -> Result<bool> {
        let url = format!(
            "{}/v1/database/{}",
            self.cfg.uri.trim_end_matches('/'),
            self.cfg.db_name
        );
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        let resp = client.get(&url).send().await?;
        Ok(resp.status().is_success())
    }

    async fn publish(self: &Arc<Self>, binary: &Path) -> Result<()> {
        if !self.cfg.module_path.exists() {
            return Err(anyhow!(
                "module_path {} does not exist; publish skipped",
                self.cfg.module_path.display()
            ));
        }

        let module_path = self.cfg.module_path.to_string_lossy().to_string();
        let server_url = self.cfg.uri.clone();
        let db_name = self.cfg.db_name.clone();

        self.logs.push(
            LogSource::Publish,
            format!(
                "[publish] {} publish -p {} -s {} --yes {}",
                binary.display(),
                module_path,
                server_url,
                db_name
            ),
        );

        run_capture(
            binary.to_string_lossy().as_ref(),
            &[
                "publish",
                "-p",
                &module_path,
                "-s",
                &server_url,
                "--yes",
                "--anonymous",
                &db_name,
            ],
            &self.logs,
            LogSource::Publish,
        )
        .await
        .with_context(|| format!("publishing module to {server_url}"))
    }

    async fn connect_sdk(self: &Arc<Self>) -> Result<()> {
        self.set_connection_state(ConnectionState::Connecting).await;

        // The notify_callback is set externally via [`make_notify_callback`]
        // before start() is called, but the SpacetimeHandle::connect API
        // requires a fresh boxed callback per connect. We re-bind here.
        let runtime = Arc::clone(self);
        let cb: Box<dyn Fn(SpacetimeNotification) + Send + Sync> =
            Box::new(move |notif: SpacetimeNotification| {
                let SpacetimeNotification::RowUpdate {
                    ref table_name,
                    ref operation,
                    ref data,
                } = notif;

                let op = match operation {
                    NotificationOp::Insert => RowOp::Insert,
                    NotificationOp::Update => RowOp::Update,
                    NotificationOp::Delete => RowOp::Delete,
                };
                let _ = runtime.events.send(RowEvent {
                    timestamp: Utc::now(),
                    table_name: table_name.clone(),
                    operation: op,
                    data_len: data.len(),
                });
            });

        match SpacetimeHandle::connect(&self.cfg.uri, &self.cfg.db_name, cb) {
            Ok(handle) => {
                self.logs.push(LogSource::Runtime, "[sdk] connected");
                self.inner.lock().await.handle = Some(handle);
                self.set_connection_state(ConnectionState::Connected).await;
                Ok(())
            }
            Err(e) => {
                self.logs
                    .push(LogSource::Runtime, format!("[sdk] connect error: {e:#}"));
                self.set_connection_state(ConnectionState::Failed).await;
                Err(e).context("SpacetimeDB SDK connect")
            }
        }
    }

    async fn disconnect_sdk(&self) {
        let mut g = self.inner.lock().await;
        // Drop the handle. The driver thread's `while let Ok(msg) = rx.recv()`
        // loop will exit when the SyncSender is dropped, terminating the
        // thread cleanly.
        g.handle = None;
        g.connection_state = ConnectionState::Disconnected;
    }

    async fn kill_child(&self) {
        let mut g = self.inner.lock().await;
        if let Some(mut child) = g.child.take() {
            // tokio::process::Child has kill_on_drop set, but we want a
            // controlled shutdown so the stdout/stderr reader tasks see EOF.
            let _ = child.start_kill();
            let _ = time::timeout(self.cfg.stop_timeout, child.wait()).await;
        }
        g.pid = None;
        g.started_at = None;
    }
}

// ───── Helpers ─────

async fn try_version(binary: &Path) -> Option<()> {
    let mut cmd = Command::new(binary);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = cmd.status().await.ok()?;
    status.success().then_some(())
}

#[cfg(target_os = "windows")]
fn well_known_install_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        out.push(PathBuf::from(&local).join("SpacetimeDB").join("spacetime.exe"));
        out.push(
            PathBuf::from(&local)
                .join("SpacetimeDB")
                .join("bin")
                .join("spacetime.exe"),
        );
    }
    out
}

#[cfg(not(target_os = "windows"))]
fn well_known_install_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        out.push(
            PathBuf::from(&home)
                .join(".local/share/spacetime/bin/spacetime"),
        );
        out.push(PathBuf::from(&home).join(".local/spacetime/bin/spacetime"));
    }
    out
}

#[cfg(target_os = "windows")]
async fn run_installer(logs: &Arc<StdbLogBroadcast>) -> Result<()> {
    run_capture(
        "powershell.exe",
        &[
            "-NoProfile",
            "-Command",
            "iwr https://windows.spacetimedb.com -useb | iex",
        ],
        logs,
        LogSource::Install,
    )
    .await
    .context("running Windows SpacetimeDB installer")
}

#[cfg(not(target_os = "windows"))]
async fn run_installer(logs: &Arc<StdbLogBroadcast>) -> Result<()> {
    run_capture(
        "sh",
        &["-c", "curl -sSf https://install.spacetimedb.com | sh"],
        logs,
        LogSource::Install,
    )
    .await
    .context("running Unix SpacetimeDB installer")
}

/// Spawn a process, stream both stdout and stderr line-by-line into the log
/// ring under the given source, return success/failure based on exit code.
async fn run_capture(
    program: &str,
    args: &[&str],
    logs: &Arc<StdbLogBroadcast>,
    source: LogSource,
) -> Result<()> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawning {program}"))?;

    if let Some(stdout) = child.stdout.take() {
        let logs = Arc::clone(logs);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                logs.push(source, line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let logs = Arc::clone(logs);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                logs.push(source, line);
            }
        });
    }

    let status = child.wait().await.context("waiting on subprocess")?;
    if !status.success() {
        return Err(anyhow!(
            "subprocess exited with {} ({})",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            program
        ));
    }
    Ok(())
}

impl infrarust_api::services::spacetimedb::ManagedSpacetimeRuntime for SpacetimeRuntime {
    fn restart(self: Arc<Self>) -> infrarust_api::event::BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            self.restart().await
        })
    }

    fn publish_now(self: Arc<Self>) -> infrarust_api::event::BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            self.publish_now().await
        })
    }

    fn status_json(&self) -> infrarust_api::event::BoxFuture<'_, serde_json::Value> {
        Box::pin(async move {
            let status = self.status().await;
            serde_json::to_value(status).unwrap_or(serde_json::Value::Null)
        })
    }

    fn subscribe_logs(&self) -> tokio::sync::broadcast::Receiver<serde_json::Value> {
        let mut rx = self.logs.subscribe();
        let (tx, rx_out) = tokio::sync::broadcast::channel(100);

        tokio::spawn(async move {
            while let Ok(entry) = rx.recv().await {
                if tx.send(serde_json::to_value(entry).unwrap_or(serde_json::Value::Null)).is_err() {
                    break;
                }
            }
        });

        rx_out
    }

    fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<serde_json::Value> {
        let mut rx = self.events.subscribe();
        let (tx, rx_out) = tokio::sync::broadcast::channel(100);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if tx.send(serde_json::to_value(event).unwrap_or(serde_json::Value::Null)).is_err() {
                    break;
                }
            }
        });

        rx_out
    }
}
