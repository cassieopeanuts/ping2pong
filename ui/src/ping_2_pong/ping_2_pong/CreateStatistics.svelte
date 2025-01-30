<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Statistics } from "./types";

const dispatch = createEventDispatcher();
let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);

export let gameId!: string;
export let signalLatency!: number;
export let scoreValidationTime!: number;
export let dhtResponseTime!: number;
export let networkDelay!: number;
export let timestamp!: number;

$: gameId, signalLatency, scoreValidationTime, dhtResponseTime, networkDelay, timestamp;
$: isStatisticsValid = true;

onMount(async () => {
  if (gameId === undefined) {
    throw new Error(`The gameId input is required for the CreateStatistics element`);
  }
  if (signalLatency === undefined) {
    throw new Error(`The signalLatency input is required for the CreateStatistics element`);
  }
  if (scoreValidationTime === undefined) {
    throw new Error(`The scoreValidationTime input is required for the CreateStatistics element`);
  }
  if (dhtResponseTime === undefined) {
    throw new Error(`The dhtResponseTime input is required for the CreateStatistics element`);
  }
  if (networkDelay === undefined) {
    throw new Error(`The networkDelay input is required for the CreateStatistics element`);
  }
  if (timestamp === undefined) {
    throw new Error(`The timestamp input is required for the CreateStatistics element`);
  }
  client = await appClientContext.getClient();
});

async function createStatistics() {
  const statisticsEntry: Statistics = {
    game_id: gameId!,
    signal_latency: signalLatency!,
    score_validation_time: scoreValidationTime!,
    dht_response_time: dhtResponseTime!,
    network_delay: networkDelay!,
    timestamp: timestamp!,
  };

  try {
    const record: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "create_statistics",
      payload: statisticsEntry,
    });
    dispatch("statistics-created", { statisticsHash: record.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<div>
  <h3>Create Statistics</h3>

  <button disabled={!isStatisticsValid} on:click={() => createStatistics()}>
    Create Statistics
  </button>
</div>
