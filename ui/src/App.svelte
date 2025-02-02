<script lang="ts">
  // Import types and functions from the Holochain client.
  import type { AppClient, HolochainError } from "@holochain/client";
  import { AppWebsocket } from "@holochain/client";
  
  // Svelte lifecycle and context functions.
  import { onMount, setContext } from "svelte";
  
  // Import the Holochain logo.
  import logo from "./assets/holochainLogo.svg";
  
  // Import our client context types and key.
  import { clientContext, type ClientContext } from "./contexts";
  
  // Local state variables.
  let client: AppClient | undefined;
  let error: HolochainError | undefined;
  let loading = false;
  
  // Define our client context object.
  // This function will lazily connect and return the Holochain client.
  const appClientContext: ClientContext = {
    getClient: async () => {
      if (!client) {
        client = await AppWebsocket.connect({
          // You can pass connection options here if needed.
          // For example: url: new URL("ws://localhost:8888")
        });
      }
      return client;
    },
  };
  
  // A helper function to establish connection.
  async function connectClient() {
    try {
      loading = true;
      client = await appClientContext.getClient();
    } catch (e) {
      error = e as HolochainError;
    } finally {
      loading = false;
    }
  }
  
  // When the component mounts, connect to the Holochain conductor.
  onMount(async () => {
    await connectClient();
  });
  
  // Make our client context available to child components.
  setContext<ClientContext>(clientContext, appClientContext);
</script>

<main>
  <div class="logo-container">
    <a href="https://developer.holochain.org/get-started/" target="_blank">
      <img src={logo} class="logo holochain" alt="Holochain logo" />
    </a>
  </div>
  <h1>Holochain Svelte hApp</h1>
  <div class="status">
    <div class="card">
      {#if loading}
        <p>Connecting...</p>
      {:else if error}
        <p class="error">{error.message}</p>
      {:else}
        <p>Client is connected.</p>
      {/if}
    </div>
    <p>
      Import scaffolded components into <code>src/App.svelte</code> to use your hApp.
    </p>
    <p class="read-the-docs">Click on the Holochain logo to learn more.</p>
  </div>
</main>

<style>
  main {
    text-align: center;
    padding: 1rem;
  }
  
  .logo-container {
    margin-bottom: 1rem;
  }
  
  .logo {
    height: 15em;
    width: auto;
    padding: 1.5em;
    will-change: filter;
    transition: filter 300ms;
  }
  
  .logo:hover {
    filter: drop-shadow(0 0 2em #646cffaa);
  }
  
  .logo.holochain:hover {
    filter: drop-shadow(0 0 2em #61dafbaa);
  }
  
  .card {
    padding: 2em;
    background-color: #f2f2f2;
    border-radius: 8px;
    margin: 0 auto;
    max-width: 400px;
  }
  
  .status p {
    margin: 1rem 0;
  }
  
  .read-the-docs {
    color: #888;
  }
  
  .error {
    color: #ff8080;
  }
</style>
