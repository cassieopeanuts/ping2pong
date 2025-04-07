// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/anchor_path.rs
use hdk::prelude::*;

// Custom entry type to wrap a Path for anchors
#[hdk_entry_helper]
#[derive(Clone, PartialEq)] // <-- Remove Eq
pub struct AnchorPath(pub Path); // Simple tuple struct wrapping Path