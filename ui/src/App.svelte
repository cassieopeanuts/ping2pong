<script lang="ts">
import type { ActionHash, AppClient, HolochainError } from "@holochain/client";
import { AppWebsocket } from "@holochain/client";
import { onMount, setContext } from "svelte";

import logo from "./assets/holochainLogo.svg";
import { type ClientContext, clientContext } from "./contexts";

let client: AppClient | undefined;
let error: HolochainError | undefined;
let loading = false;

const appClientContext = {
  getClient: async () => {
    if (!client) {
      client = await AppWebsocket.connect();
    }
    return client;
  },
};

onMount(async () => {
  try {
    loading = true;
    client = await appClientContext.getClient();
  } catch (e) {
    error = e as HolochainError;
  } finally {
    loading = false;
  }
});

setContext<ClientContext>(clientContext, appClientContext);
</script>

<main>
  <div>
    <a href="https://developer.holochain.org/get-started/" target="_blank">
      <img src={logo} class="logo holochain" alt="holochain logo" />
    </a>
  </div>
  <h1>Holochain Svelte hApp</h1>
  <div>
    <div class="card">
      {#if loading}
        <p>connecting...</p>
      {:else if error}
        <p>{error.message}</p>
      {:else}
        <p>Client is connected.</p>
      {/if}
    </div>
    <p>
      Import scaffolded components into <code>src/App.svelte</code> to use your
      hApp
    </p>
    <p class="read-the-docs">Click on the Holochain logo to learn more</p>
  </div>
</main>

<style>
.logo {
  height: 15em;
  padding: 1.5em;
  will-change: filter;
  transition: filter 300ms;
  width: auto;
}

.logo:hover {
  filter: drop-shadow(0 0 2em #646cffaa);
}

.logo.holochain:hover {
  filter: drop-shadow(0 0 2em #61dafbaa);
}

.card {
  padding: 2em;
}

.read-the-docs {
  color: #888;
}
</style>
