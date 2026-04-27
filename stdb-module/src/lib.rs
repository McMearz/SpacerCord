//! Default SpacerCord SpacetimeDB module.
//!
//! Bundled minimal schema: a single `player_profile` table keyed by Minecraft
//! UUID, plus an `ensure_player_profile` reducer that the proxy invokes on
//! `PostLoginEvent`. Operators can extend or replace this module — the proxy
//! only requires the `ensure_player_profile(uuid, username)` reducer.

use spacetimedb::{ReducerContext, Table, reducer, table};

#[table(accessor = player_profile, public)]
pub struct PlayerProfile {
    #[primary_key]
    pub uuid: String,
    pub username: String,
}

#[reducer]
pub fn ensure_player_profile(ctx: &ReducerContext, uuid: String, username: String) {
    if let Some(existing) = ctx.db.player_profile().uuid().find(&uuid) {
        if existing.username != username {
            ctx.db.player_profile().uuid().update(PlayerProfile {
                uuid: existing.uuid.clone(),
                username,
            });
        }
        return;
    }

    ctx.db.player_profile().insert(PlayerProfile { uuid, username });
}
