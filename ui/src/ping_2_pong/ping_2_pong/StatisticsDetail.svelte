<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import EditStatistics from "./EditStatistics.svelte";
import type { Statistics } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

let loading: boolean = false;
let editing = false;
let error: HolochainError | undefined;
let record: Record | undefined;
let statistics: Statistics | undefined;

export let statisticsHash: ActionHash;

$: editing, error, loading, record, statistics;

onMount(async () => {
  if (statisticsHash === undefined) {
    throw new Error(`The statisticsHash input is required for the StatisticsDetail element`);
  }
  client = await appClientContext.getClient();
  await fetchStatistics();
});

async function fetchStatistics() {
  loading = true;
  try {
    record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "get_latest_statistics",
      payload: statisticsHash,
    });
    if (record) {
      statistics = decode((record.entry as any).Present.entry) as Statistics;
    }
  } catch (e) {
    error = e as HolochainError;
  } finally {
    loading = false;
  }
}

async function deleteStatistics() {
  try {
    await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "delete_statistics",
      payload: statisticsHash,
    });
    dispatch("statistics-deleted", { statisticsHash: statisticsHash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

{#if loading}
  <progress />
{:else if error}
  <div class="alert">Error fetching the statistics: {error.message}</div>
{:else if editing}
  <EditStatistics
    originalStatisticsHash={statisticsHash}
    currentRecord={record}
    on:statistics-updated={async () => {
      editing = false;
      await fetchStatistics();
    }}
    on:edit-canceled={() => {
      editing = false;
    }}
  />
{:else}
  <section>
    <div>
      <button
        on:click={() => {
          editing = true;
        }}
      >edit</button>
      <button on:click={() => deleteStatistics()}>delete</button>
    </div>
  </section>
{/if}
