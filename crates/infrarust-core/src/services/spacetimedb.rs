//! SpacetimeDB service implementation.
//!
//! This crate provides the concrete implementation of the [`SpacetimeService`]
//! trait defined in `infrarust-api`.

use infrarust_api::services::spacetimedb::SpacetimeService;

#[cfg(feature = "spacetimedb")]
use std::sync::Arc;

#[cfg(feature = "spacetimedb")]
use infrarust_spacetimedb::SpacetimeRuntime;

/// Concrete implementation of [`SpacetimeService`].
///
/// This implementation wraps the managed [`SpacetimeRuntime`] which owns the
/// lifecycle of the SpacetimeDB child process and its SDK connection.
#[cfg(feature = "spacetimedb")]
pub struct SpacetimeServiceImpl {
    runtime: Arc<SpacetimeRuntime>,
}

#[cfg(feature = "spacetimedb")]
impl SpacetimeServiceImpl {
    /// Creates a new service instance from a runtime handle.
    pub fn new(runtime: Arc<SpacetimeRuntime>) -> Self {
        Self { runtime }
    }
}

#[cfg(feature = "spacetimedb")]
impl SpacetimeService for SpacetimeServiceImpl {
    fn ensure_player_profile(&self, uuid: String, username: String) {
        // Since we are fire-and-forget, if the handle is currently None (e.g. restarting)
        // we just drop the call. The runtime will log that the connection is down.
        let runtime = Arc::clone(&self.runtime);
        tokio::spawn(async move {
            if let Some(handle) = runtime.handle().await {
                handle.ensure_player_profile(uuid, username);
            }
        });
    }

    fn call_reducer(&self, reducer_name: &str, args: Vec<u8>) {
        let runtime = Arc::clone(&self.runtime);
        let reducer_name = reducer_name.to_string();
        tokio::spawn(async move {
            if let Some(handle) = runtime.handle().await {
                handle.call_reducer(reducer_name, args);
            }
        });
    }

    fn subscribe(&self, query: &str) {
        let runtime = Arc::clone(&self.runtime);
        let query = query.to_string();
        tokio::spawn(async move {
            if let Some(handle) = runtime.handle().await {
                handle.subscribe(query);
            }
        });
    }
}

/// A no-op implementation of [`SpacetimeService`].
///
/// Used when the `spacetimedb` feature is disabled or the database connection
/// fails. This prevents plugins from crashing if they attempt to use the service.
pub struct NoopSpacetimeService;

impl SpacetimeService for NoopSpacetimeService {
    fn ensure_player_profile(&self, _uuid: String, _username: String) {
        // Silently do nothing.
    }

    fn call_reducer(&self, _reducer_name: &str, _args: Vec<u8>) {
        // Silently do nothing.
    }

    fn subscribe(&self, _query: &str) {
        // Silently do nothing.
    }
}
