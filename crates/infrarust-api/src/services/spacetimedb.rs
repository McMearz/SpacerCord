//! SpacetimeDB service trait.
//!
//! This crate defines the bridge between Infrarust and SpacetimeDB.
//! It allows plugins to persist data and react to database changes
//! without depending on the SpacetimeDB SDK directly.

/// Service for interacting with SpacetimeDB.
///
/// This service is provided to plugins via the [`PluginContext`](crate::plugin::PluginContext).
/// It follows a "fire-and-forget" pattern where calls are queued and sent
/// asynchronously to a dedicated database driver thread to ensure proxy
/// performance is never impacted by database latency.
pub trait SpacetimeService: Send + Sync {
    /// Fires `ensure_player_profile` for the given player.
    ///
    /// This is a specialized helper for the common task of ensuring a player
    /// exists in the database schema.
    ///
    /// The call is queued and sent asynchronously to SpacetimeDB.
    fn ensure_player_profile(&self, uuid: String, username: String);

    /// Calls a custom reducer by name with encoded arguments.
    ///
    /// This is the primary extension point for custom game logic. Plugin
    /// developers can define reducers in their SpacetimeDB modules and
    /// call them here using BSATN or JSON encoded arguments.
    ///
    /// # Example
    /// ```ignore
    /// let args = bsatn::to_vec(&(player_uuid, item_id)).unwrap();
    /// stdb.call_reducer("give_item", args);
    /// ```
    fn call_reducer(&self, reducer_name: &str, args: Vec<u8>);

    /// Subscribes to SpacetimeDB updates using a SQL-like query.
    ///
    /// Once subscribed, any changes to rows matching this query (Insert, Update, Delete)
    /// will trigger a [`SpacetimeRowEvent`](crate::events::spacetimedb::SpacetimeRowEvent)
    /// on the global event bus.
    ///
    /// This allows plugins to react to database state changes initiated by
    /// other plugins, web dashboards, or external tools.
    fn subscribe(&self, query: &str);
}
