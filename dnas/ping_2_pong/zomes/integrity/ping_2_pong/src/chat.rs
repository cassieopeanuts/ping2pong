use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ChatMessage {
    pub sender: AgentPubKey,
    pub content: String,
    pub timestamp: Timestamp,
}
