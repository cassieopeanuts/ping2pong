use hdk::prelude::*;

// Statistics entry recorded after a game finishes.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Statistics {
    pub game_id: ActionHash, // Links back to the original Game create action
    pub timestamp: Timestamp, // When stats were recorded

    // Example metrics (measured client-side, validated server-side for sanity)
    pub signal_latency: u32, // e.g., average round-trip signal time in ms
    // Consider renaming or removing this if DHT validation time isn't relevant/measurable
    pub score_validation_time: u32, // Time for score entry to be validated/committed? (Hard to measure) - RENAME? -> post_game_commit_time?
    pub dht_response_time: u32, // Average time for DHT gets? (Client measured)
    pub network_delay: u32, // Estimated network RTT? (Client measured)
}