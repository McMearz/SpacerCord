//! SpacetimeDB Engine for SpacerCord.
//!
//! This crate manages the low-level connection to SpacetimeDB and provides
//! a high-performance, async-safe bridge to the rest of the proxy.
//!
//! # Architecture
//!
//! To prevent database latency from blocking the Minecraft protocol (which
//! is extremely sensitive to jitter), this crate runs a dedicated **OS thread**
//! (`stdb-driver`).
//!
//! 1. **Outbound**: The proxy and plugins send messages to the driver via a
//!    bounded MPSC channel. The driver thread processes these sequentially.
//! 2. **Inbound**: The driver thread listens to the SpacetimeDB SDK's event
//!    stream and executes a callback that dispatches events back into the
//!    Tokio-based global EventBus.

pub mod module_bindings;

use module_bindings::{
    DbConnection,
    ensure_player_profile_reducer::ensure_player_profile as EnsurePlayerProfileExt,
};
use std::sync::mpsc;
use std::sync::Arc;

/// Internal message types processed by the `stdb-driver` thread.
enum DriverMsg {
    /// Specialized call for ensuring a player exists.
    EnsurePlayerProfile { uuid: String, username: String },
    /// Generic call for any reducer defined in the DB module.
    CallReducer { name: String, args: Vec<u8> },
    /// SQL-based subscription for real-time table updates.
    Subscribe { query: String },
}

/// Generic notification sent from the driver thread back to the proxy core.
pub enum SpacetimeNotification {
    /// A row in a subscribed table has changed.
    RowUpdate {
        table_name: String,
        operation: NotificationOp,
        data: Vec<u8>,
    },
}

/// Simplified operation types for row events.
pub enum NotificationOp {
    Insert,
    Update,
    Delete,
}

/// Thread-safe handle for interacting with SpacetimeDB.
///
/// This is the concrete implementation that powers the `SpacetimeService` API.
/// It can be cloned freely as it only contains a channel sender.
#[derive(Clone)]
pub struct SpacetimeHandle {
    tx: mpsc::SyncSender<DriverMsg>,
}

impl SpacetimeHandle {
    /// Establishes a connection to SpacetimeDB and spawns the driver thread.
    ///
    /// # Arguments
    /// * `uri` - The WebSocket URI of the SpacetimeDB host (e.g. `http://localhost:3000`).
    /// * `db_name` - The name of the database module to join.
    /// * `notify_callback` - A closure called by the driver thread when database events occur.
    pub fn connect(
        uri: &str, 
        db_name: &str,
        notify_callback: Box<dyn Fn(SpacetimeNotification) + Send + Sync>,
    ) -> anyhow::Result<Self> {
        // Bounded channel provides back-pressure: if the driver thread falls
        // behind, callers will block or receive an error rather than
        // exhausting proxy memory.
        let (tx, rx) = mpsc::sync_channel::<DriverMsg>(1024);

        let uri = uri.to_string();
        let db_name = db_name.to_string();

        std::thread::Builder::new()
            .name("stdb-driver".to_string())
            .spawn(move || driver_main(uri, db_name, rx, notify_callback))?;

        Ok(Self { tx })
    }

    /// Queues an `ensure_player_profile` call.
    pub fn ensure_player_profile(&self, uuid: String, username: String) {
        let _ = self.tx.try_send(DriverMsg::EnsurePlayerProfile { uuid, username });
    }

    /// Queues a generic reducer call.
    pub fn call_reducer(&self, name: String, args: Vec<u8>) {
        let _ = self.tx.try_send(DriverMsg::CallReducer { name, args });
    }

    /// Queues a SQL subscription request.
    pub fn subscribe(&self, query: String) {
        let _ = self.tx.try_send(DriverMsg::Subscribe { query });
    }
}

/// The main loop for the database driver thread.
fn driver_main(
    uri: String, 
    db_name: String, 
    rx: mpsc::Receiver<DriverMsg>,
    notify: Box<dyn Fn(SpacetimeNotification) + Send + Sync>,
) {
    let notify_arc = Arc::new(notify);

    // Initialize the SDK connection.
    let conn = match DbConnection::builder()
        .with_uri(&uri)
        .with_database_name(&db_name)
        .on_connect(|_ctx, identity, _token| {
            tracing::info!(identity = %identity, "connected to SpacetimeDB");
        })
        .on_event({
            let notify = Arc::clone(&notify_arc);
            move |_ctx, event| {
                // Map low-level SDK transaction events to our generic Notification type.
                if let spacetimedb_sdk::Event::Transaction(tx) = event {
                    for row_event in &tx.row_events {
                        let op = match row_event.op {
                            spacetimedb_sdk::RowOp::Insert => NotificationOp::Insert,
                            spacetimedb_sdk::RowOp::Update { .. } => NotificationOp::Update,
                            spacetimedb_sdk::RowOp::Delete => NotificationOp::Delete,
                        };

                        notify(SpacetimeNotification::RowUpdate {
                            table_name: row_event.table_name.clone(),
                            operation: op,
                            data: row_event.row_data.clone(),
                        });
                    }
                }
            }
        })
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to SpacetimeDB");
            return;
        }
    };

    // Run the SDK's internal background thread for WebSocket maintenance.
    let _sdk_thread = conn.run_threaded();

    // Process outgoing messages from the proxy core.
    while let Ok(msg) = rx.recv() {
        match msg {
            DriverMsg::EnsurePlayerProfile { uuid, username } => {
                let _ = EnsurePlayerProfileExt::ensure_player_profile(&conn.reducers, uuid, username);
            }
            DriverMsg::CallReducer { name, args } => {
                let _ = conn.invoke_reducer(&name, args);
            }
            DriverMsg::Subscribe { query } => {
                let _ = conn.subscribe(&[&query]);
            }
        }
    }

    tracing::info!("SpacetimeDB driver thread exiting");
}
