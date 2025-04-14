// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/lib.rs
pub mod game;
pub mod player;
pub mod score;
pub mod statistics;
pub mod utils;

use hdk::prelude::*;
use ping_2_pong_integrity::*;


#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}

// Signal enum definition (Requires LinkTypes to have Serde derives)
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Signal {
    LinkCreated { action: SignedActionHashed, link_type: LinkTypes },
    LinkDeleted { action: SignedActionHashed, create_link_action: SignedActionHashed, link_type: LinkTypes },
    EntryCreated { action: SignedActionHashed, app_entry: EntryTypes },
    EntryUpdated { action: SignedActionHashed, app_entry: EntryTypes, original_app_entry: EntryTypes },
    EntryDeleted { action: SignedActionHashed, original_app_entry: EntryTypes },
    GameInvitation {
        game_id: ActionHash,
        inviter: AgentPubKey,
        message: String,
    },
    GameStarted {
        game_id: ActionHash,
        opponent: AgentPubKey, // The pubkey of the other player in the game
    },
}

#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}

// signal_action: Helper for post_commit
fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
     // Use action() directly, no .hashed field needed here
     match action.action().clone() {
        Action::CreateLink(create_link) => {
            // Assuming LinkTypes::from_type works as intended
            if let Ok(Some(link_type)) = LinkTypes::from_type(create_link.zome_index, create_link.link_type) {
                emit_signal(Signal::LinkCreated { action, link_type })?;
            }
            Ok(())
        }
        Action::DeleteLink(delete_link) => {
            match get(delete_link.link_add_address.clone(), GetOptions::default())? {
                Some(record) => {
                    // Check the action content directly
                    if let Action::CreateLink(create_link) = record.action().clone() {
                        if let Ok(Some(link_type)) = LinkTypes::from_type(create_link.zome_index, create_link.link_type) {
                            emit_signal(Signal::LinkDeleted {
                                action, // The DeleteLink action
                                link_type,
                                create_link_action: record.signed_action.clone(), // The original CreateLink action
                            })?;
                        }
                    } else {
                         warn!("Original action for DeleteLink signal is not CreateLink: {:?}", delete_link.link_add_address);
                    }
                }
                None => {
                    warn!("Could not find matching CreateLink record for DeleteLink signal: {:?}", delete_link.link_add_address);
                }
            }
            Ok(())
        }
        Action::Create(create) => {
            // FIX: Directly match on create.entry_type, it's not an Option
            if let EntryType::App(_) = create.entry_type {
                 match get_entry_for_action(&action.hashed.hash) { // Use action hash here
                    Ok(Some(app_entry)) => {
                        emit_signal(Signal::EntryCreated { action, app_entry })?;
                    },
                    Ok(None) => {
                         debug!("Could not get entry for signal EntryCreated: {}", action.hashed.hash);
                    },
                    Err(e) => {
                        error!("Error getting entry for signal EntryCreated: {:?}", e);
                    }
                 }
             }
            Ok(())
        }
        Action::Update(update) => {
             let maybe_app_entry = get_entry_for_action(&action.hashed.hash);
             let maybe_original_app_entry = get_entry_for_action(&update.original_action_address);
             match (maybe_app_entry, maybe_original_app_entry) {
                 (Ok(Some(app_entry)), Ok(Some(original_app_entry))) => {
                      emit_signal(Signal::EntryUpdated {
                         action,
                         app_entry,
                         original_app_entry,
                     })?;
                 },
                 (Ok(_), Ok(_)) => {
                     debug!("Could not get entries for signal EntryUpdated: new: {}, original: {}", action.hashed.hash, update.original_action_address);
                 },
                 (Err(e), _) | (_, Err(e)) => {
                      error!("Error getting entries for signal EntryUpdated: {:?}", e);
                 }
             }
             Ok(())
        }
        Action::Delete(delete) => {
             match get_entry_for_action(&delete.deletes_address) {
                 Ok(Some(original_app_entry)) => {
                     emit_signal(Signal::EntryDeleted {
                         action,
                         original_app_entry,
                     })?;
                 },
                 Ok(None) => {
                     debug!("Could not get entry for signal EntryDeleted: {}", delete.deletes_address);
                 },
                 Err(e) => {
                      error!("Error getting entry for signal EntryDeleted: {:?}", e);
                 }
             }
            Ok(())
        }
        _ => Ok(()),
    }
}

// get_entry_for_action: Helper for post_commit
fn get_entry_for_action(action_hash: &ActionHash) -> ExternResult<Option<EntryTypes>> {
     // Fetching details might be more robust than just get record
     let details = get_details(action_hash.clone(), GetOptions::default())?;
     let record = match details {
        Some(Details::Record(record_details)) => record_details.record,
        _ => return Ok(None), // No record found or other detail type
    };
    let entry = match record.entry().as_option() {
        Some(entry) => entry.clone(),
        None => return Ok(None),
    };
     let action = record.action();
     let (zome_index, entry_index) = match action.entry_type() {
        // Use .clone() on fields if necessary
        Some(EntryType::App(AppEntryDef { zome_index, entry_index, .. })) => (*zome_index, *entry_index),
        _ => return Ok(None),
    };

     match EntryTypes::deserialize_from_type(zome_index, entry_index, &entry) {
         Ok(Some(entry_type)) => Ok(Some(entry_type)),
         Ok(None) => {
             // FIX: Use debug formatting for indexes
             warn!("Could not deserialize entry type for action {:?} with type index ({:?}, {:?})", action_hash, zome_index, entry_index);
             Ok(None)
         },
         Err(e) => {
             error!("Failed to deserialize entry for action {:?}: {:?}", action_hash, e);
             Err(e.into())
         }
     }
}

// send_signal: Generic function for UI to send signals
#[hdk_extern]
pub fn send_signal(signal: SerializedBytes) -> ExternResult<()> {
    emit_signal(&signal)?;
    Ok(())
}