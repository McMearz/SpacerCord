//! SpacetimeDB row events.
//!
//! These events allow plugins to react to real-time database changes.

use crate::event::Event;

/// Operation performed on a SpacetimeDB row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowOperation {
    /// A new row was added to the table.
    Insert,
    /// An existing row was modified.
    Update,
    /// A row was removed from the table.
    Delete,
}

/// Fired when a subscribed SpacetimeDB table receives an update.
///
/// This event is dispatched whenever a change occurs in SpacetimeDB that
/// matches a subscription created via [`SpacetimeService::subscribe`](crate::services::spacetimedb::SpacetimeService::subscribe).
pub struct SpacetimeRowEvent {
    /// Name of the table that changed.
    pub table_name: String,
    /// Type of operation (Insert, Update, Delete).
    pub operation: RowOperation,
    /// Raw row data. 
    /// 
    /// If the operation is `Update`, this usually contains the **new** state of the row.
    /// The data is typically BSATN encoded, but can be JSON depending on the proxy configuration.
    pub row_data: Vec<u8>,
}

impl Event for SpacetimeRowEvent {}
