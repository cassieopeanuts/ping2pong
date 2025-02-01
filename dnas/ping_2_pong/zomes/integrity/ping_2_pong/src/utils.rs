use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct AnchorString(pub String);

// Helper: Create an anchor hash from a string
pub fn anchor_for(input: &str) -> ExternResult<AnyLinkableHash> {
    let anchor = AnchorString(input.to_string());
    let hash = hash_entry(&anchor)?;
    Ok(hash.into())
}