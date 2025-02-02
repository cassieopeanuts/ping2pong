// This element was auto created by Holochain scaffolding tool and we dont really need it
// We keep it for archive purpose

<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Statistics } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

export let currentRecord!: Record;
export let originalStatisticsHash!: ActionHash;

let currentStatistics: Statistics = decode((currentRecord.entry as any).Present.entry) as Statistics;

$: ;
$: isStatisticsValid = true;

onMount(async () => {
  if (!currentRecord) {
    throw new Error(`The currentRecord input is required for the EditStatistics element`);
  }
  if (!originalStatisticsHash) {
    throw new Error(`The originalStatisticsHash input is required for the EditStatistics element`);
  }
  client = await appClientContext.getClient();
});

async function updateStatistics() {
  const statistics: Statistics = {
    game_id: currentStatistics.game_id,
    signal_latency: currentStatistics.signal_latency,
    score_validation_time: currentStatistics.score_validation_time,
    dht_response_time: currentStatistics.dht_response_time,
    network_delay: currentStatistics.network_delay,
    timestamp: currentStatistics.timestamp,
  };

  try {
    const updateRecord: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "update_statistics",
      payload: {
        original_statistics_hash: originalStatisticsHash,
        previous_statistics_hash: currentRecord.signed_action.hashed.hash,
        updated_statistics: statistics,
      },
    });

    dispatch("statistics-updated", { actionHash: updateRecord.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<section>
  <div>
    <button on:click={() => dispatch("edit-canceled")}>Cancel</button>
    <button disabled={!isStatisticsValid} on:click={() => updateStatistics()}>
      Edit Statistics
    </button>
  </div>
</section>
