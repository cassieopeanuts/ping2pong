<script lang="ts">
  import { createEventDispatcher, getContext } from "svelte";
  import { playerProfile } from "../stores/playerProfile";
  import { clientContext, type ClientContext } from "../contexts";
  import type { AppClient, AgentPubKey } from "@holochain/client";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../holochainConfig";
  // Remove encodeHashToBase64 if not needed here
  // import { encodeHashToBase64 } from "@holochain/client";

  const dispatch = createEventDispatcher();
  let nickname: string = "";
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);
  let errorMessage: string | null = null;
  let isLoading: boolean = false;

  async function register() {
    if (nickname.trim() === "") {
      errorMessage = "Nickname cannot be empty.";
      return;
    }
    isLoading = true;
    errorMessage = null;
    try {
      client = await appClientContext.getClient();
      const agentKey: AgentPubKey = client.myPubKey; // Get the raw AgentPubKey

      const playerPayload = { player_key: agentKey, player_name: nickname.trim() };

      const record = await client.callZome({
        cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME,
        fn_name: "create_player", payload: playerPayload,
      });
      console.log("Player created:", record);

      // FIX: Set profile store with the raw AgentPubKey
      playerProfile.set({
        agentKey: agentKey, // Store the object/Uint8Array
        nickname: nickname.trim()
      });

      dispatch("registered", { nickname: nickname.trim() });
    } catch (e: any) {
      console.error("Registration error:", e);
      errorMessage = e.data?.data || e.message || "Registration failed. Please try again.";
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="welcome-popup">
  <div class="welcome-popup-content">
    <h2>Welcome! Let's Pong to Ping!</h2>
    <input type="text" placeholder="Enter your nickname" bind:value={nickname} disabled={isLoading} />
    <button on:click={register} disabled={isLoading}>
      {#if isLoading}Registering...{:else}Register{/if}
    </button>
    {#if errorMessage}
      <p class="error-message" style="margin-top: 1rem;">{errorMessage}</p>
    {/if}
  </div>
</div>

<style>
  h2 {
    font-size: 1.5rem; /* Default 16px * 1.5 = 24px. Adjusted from global 2.2em (35.2px) */
    line-height: 1.2; /* Ensure consistent line height for the new font size */
    /* The global h2 style in index.css has line-height: 1.2, so this might be redundant if not changing it,
       but explicitly setting it here ensures it's considered for this specific component's h2. */
  }
  /* Other elements like input, button, p.error-message already have 1rem/1em font-size via index.css,
     which should be acceptable for 'Press Start 2P'. */
</style>
