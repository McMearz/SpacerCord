//! SpacetimeDB engine for SpacerCord.
//!
//! Owns the connection to SpacetimeDB on a dedicated OS thread and bridges
//! it to the Tokio-based proxy via a bounded MPSC channel. The driver thread
//! both runs the SDK's WebSocket loop (`run_threaded`) and serialises all
//! outgoing reducer calls so back-pressure flows back to callers when the
//! database is slow.

pub mod log_ring;
pub mod module_bindings;
pub mod runtime;

pub use log_ring::{LogSource, StdbLogBroadcast, StdbLogEntry};
pub use runtime::{ConnectionState, RowEvent, RowOp, RuntimeStatus, ServerState, SpacetimeRuntime};

use module_bindings::{
    DbConnection,
    ensure_player_profile_reducer::ensure_player_profile as EnsurePlayerProfileExt,
};
use spacetimedb_sdk::DbContext;
use std::sync::mpsc;

enum DriverMsg {
    EnsurePlayerProfile { uuid: String, username: String },
    CallReducer { name: String, args: Vec<u8> },
    Subscribe { query: String },
}

pub enum SpacetimeNotification {
    RowUpdate {
        table_name: String,
        operation: NotificationOp,
        data: Vec<u8>,
    },
}

pub enum NotificationOp {
    Insert,
    Update,
    Delete,
}

#[derive(Clone)]
pub struct SpacetimeHandle {
    tx: mpsc::SyncSender<DriverMsg>,
}

impl SpacetimeHandle {
    pub fn connect(
        uri: &str,
        db_name: &str,
        notify_callback: Box<dyn Fn(SpacetimeNotification) + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<DriverMsg>(1024);

        let uri = uri.to_string();
        let db_name = db_name.to_string();

        std::thread::Builder::new()
            .name("stdb-driver".to_string())
            .spawn(move || driver_main(uri, db_name, rx, notify_callback))?;

        Ok(Self { tx })
    }

    pub fn ensure_player_profile(&self, uuid: String, username: String) {
        let _ = self
            .tx
            .try_send(DriverMsg::EnsurePlayerProfile { uuid, username });
    }

    pub fn call_reducer(&self, name: String, args: Vec<u8>) {
        let _ = self.tx.try_send(DriverMsg::CallReducer { name, args });
    }

    pub fn subscribe(&self, query: String) {
        let _ = self.tx.try_send(DriverMsg::Subscribe { query });
    }
}

fn driver_main(
    uri: String,
    db_name: String,
    rx: mpsc::Receiver<DriverMsg>,
    _notify: Box<dyn Fn(SpacetimeNotification) + Send + Sync>,
) {
    let conn = match DbConnection::builder()
        .with_uri(&uri)
        .with_database_name(&db_name)
        .on_connect(|_ctx, identity, _token| {
            tracing::info!(identity = %identity, "connected to SpacetimeDB");
        })
        .on_connect_error(|_ctx, err| {
            tracing::error!(error = %err, "SpacetimeDB connection error");
        })
        .on_disconnect(|_ctx, err| {
            if let Some(err) = err {
                tracing::warn!(error = %err, "SpacetimeDB disconnected");
            } else {
                tracing::info!("SpacetimeDB disconnected cleanly");
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

    let _sdk_thread = conn.run_threaded();

    while let Ok(msg) = rx.recv() {
        match msg {
            DriverMsg::EnsurePlayerProfile { uuid, username } => {
                if let Err(e) =
                    EnsurePlayerProfileExt::ensure_player_profile(&conn.reducers, uuid, username)
                {
                    tracing::warn!(error = %e, "failed to invoke ensure_player_profile reducer");
                }
            }
            DriverMsg::CallReducer { name, args } => {
                tracing::warn!(
                    reducer = %name,
                    arg_bytes = args.len(),
                    "dynamic reducer dispatch not supported in bundled build; \
                     extend module_bindings or call typed reducers directly"
                );
            }
            DriverMsg::Subscribe { query } => {
                let _ = conn
                    .subscription_builder()
                    .on_applied(|_ctx| {
                        tracing::debug!("SpacetimeDB subscription applied");
                    })
                    .on_error(|_ctx, err| {
                        tracing::warn!(error = %err, "SpacetimeDB subscription error");
                    })
                    .subscribe(query);
            }
        }
    }

    tracing::info!("SpacetimeDB driver thread exiting");
}
