use hdk::prelude::*;

// Custom entry type to wrap a Path for anchors
#[hdk_entry_helper]
#[derive(Clone, PartialEq)] 
pub struct AnchorPath(pub Path); // Simple tuple struct wrapping Path