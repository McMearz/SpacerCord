//! Bounded log ring + broadcast channel for the SpacetimeDB child process.
//!
//! Mirrors the pattern in `infrarust-plugin-admin-api/src/log_layer.rs` so the
//! admin panel can stream installer / `spacetime start` / `spacetime publish`
//! output as a single live console.

use std::collections::VecDeque;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Where a console line came from, so the panel can colour-code or filter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
    /// `spacetime start` stdout.
    Server,
    /// `spacetime start` stderr.
    ServerErr,
    /// `spacetime publish` (build + push) output.
    Publish,
    /// CLI auto-installer output.
    Install,
    /// Runtime supervisor messages (spawn / shutdown / port-kill / readiness).
    Runtime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdbLogEntry {
    pub timestamp: DateTime<Utc>,
    pub source: LogSource,
    pub line: String,
}

/// Shared log buffer: a bounded broadcast (lossy for slow subscribers) plus a
/// mutex-guarded ring of the most recent N lines for REST snapshot endpoints.
pub struct StdbLogBroadcast {
    pub tx: broadcast::Sender<StdbLogEntry>,
    history: Mutex<VecDeque<StdbLogEntry>>,
    history_max: usize,
}

impl StdbLogBroadcast {
    pub fn new(channel_capacity: usize, history_max: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self {
            tx,
            history: Mutex::new(VecDeque::with_capacity(history_max)),
            history_max,
        }
    }

    /// Append a line to history and broadcast it to live subscribers.
    pub fn push(&self, source: LogSource, line: impl Into<String>) {
        let entry = StdbLogEntry {
            timestamp: Utc::now(),
            source,
            line: line.into(),
        };
        if let Ok(mut h) = self.history.lock() {
            if h.len() >= self.history_max {
                h.pop_front();
            }
            h.push_back(entry.clone());
        }
        // broadcast::send returns an error only when there are no subscribers,
        // which we don't care about — the history ring has captured the line.
        let _ = self.tx.send(entry);
    }

    pub fn history_snapshot(&self) -> Vec<StdbLogEntry> {
        self.history
            .lock()
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StdbLogEntry> {
        self.tx.subscribe()
    }
}
