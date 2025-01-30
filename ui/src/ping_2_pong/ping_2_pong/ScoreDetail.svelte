<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import EditScore from "./EditScore.svelte";
import type { Score } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

let loading: boolean = false;
let editing = false;
let error: HolochainError | undefined;
let record: Record | undefined;
let score: Score | undefined;

export let scoreHash: ActionHash;

$: editing, error, loading, record, score;

onMount(async () => {
  if (scoreHash === undefined) {
    throw new Error(`The scoreHash input is required for the ScoreDetail element`);
  }
  client = await appClientContext.getClient();
  await fetchScore();
});

async function fetchScore() {
  loading = true;
  try {
    record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "get_latest_score",
      payload: scoreHash,
    });
    if (record) {
      score = decode((record.entry as any).Present.entry) as Score;
    }
  } catch (e) {
    error = e as HolochainError;
  } finally {
    loading = false;
  }
}

async function deleteScore() {
  try {
    await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "delete_score",
      payload: scoreHash,
    });
    dispatch("score-deleted", { scoreHash: scoreHash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

{#if loading}
  <progress />
{:else if error}
  <div class="alert">Error fetching the score: {error.message}</div>
{:else if editing}
  <EditScore
    originalScoreHash={scoreHash}
    currentRecord={record}
    on:score-updated={async () => {
      editing = false;
      await fetchScore();
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
      <button on:click={() => deleteScore()}>delete</button>
    </div>
  </section>
{/if}
