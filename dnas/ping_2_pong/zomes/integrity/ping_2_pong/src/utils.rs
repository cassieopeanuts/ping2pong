// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/utils.rs
use hdk::prelude::*;
// Import the necessary types from the main lib.rs
use crate::{AnchorPath, EntryTypes};

// Function to create a deterministic anchor hash from a string using Path entry creation.
// Used for global anchors ("games", "presence") and dynamic ones (player names).
pub fn anchor_for(input: &str) -> ExternResult<AnyLinkableHash> {
    // 1. Create the Path object
    let path = Path::from(input);

    // 2. Check if the AnchorPath entry already exists to avoid duplicates
    //    We can use the path's entry hash to try and fetch it.
    let path_hash = path.path_entry_hash()?;
    // <-- FIX: Replace latest() with network()
    let maybe_record = get(path_hash.clone(), GetOptions::network())?;

    if maybe_record.is_none() {
        // 3. If it doesn't exist, create the AnchorPath entry
        create_entry(&EntryTypes::AnchorPath(AnchorPath(path.clone())))?;
        debug!("Created AnchorPath entry for: {}", input);
    } else {
        debug!("AnchorPath entry already exists for: {}", input);
    }

    // 4. Return the path's entry hash wrapped in AnyLinkableHash
    Ok(AnyLinkableHash::from(path_hash))
}