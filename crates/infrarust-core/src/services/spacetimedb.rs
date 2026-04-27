//! SpacetimeDB service implementation.
//!
//! This crate provides the concrete implementation of the [`SpacetimeService`]
//! trait defined in `infrarust-api`.

use infrarust_api::services::spacetimedb::SpacetimeService;

#[cfg(feature = "spacetimedb")]
use infrarust_spacetimedb::SpacetimeHandle;

/// Concrete implementation of [`SpacetimeService`].
///
/// This implementation wraps the thread-safe [`SpacetimeHandle`] from the
/// `infrarust-spacetimedb` crate.
#[cfg(feature = "spacetimedb")]
pub struct SpacetimeServiceImpl {
    handle: SpacetimeHandle,
}

#[cfg(feature = "spacetimedb")]
impl SpacetimeServiceImpl {
    /// Creates a new service instance from a driver handle.
    pub fn new(handle: SpacetimeHandle) -> Self {
        Self { handle }
    }
}

#[cfg(feature = "spacetimedb")]
impl SpacetimeService for SpacetimeServiceImpl {
    fn ensure_player_profile(&self, uuid: String, username: String) {
        self.handle.ensure_player_profile(uuid, username);
    }

    fn call_reducer(&self, reducer_name: &str, args: Vec<u8>) {
        self.handle.call_reducer(reducer_name.to_string(), args);
    }

    fn subscribe(&self, query: &str) {
        self.handle.subscribe(query.to_string());
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
