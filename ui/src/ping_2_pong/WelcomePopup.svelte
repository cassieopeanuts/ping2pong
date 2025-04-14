<script lang="ts">
  import { createEventDispatcher, getContext } from "svelte";
  import { playerProfile } from "../stores/playerProfile";
  import { clientContext, type ClientContext } from "../contexts";
  import type { AppClient, AgentPubKey } from "@holochain/client";
  // Remove encodeHashToBase64 if not needed here
  // import { encodeHashToBase64 } from "@holochain/client";

  const dispatch = createEventDispatcher();
  let nickname: string = "";
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  async function register() {
    if (nickname.trim() === "") { /* ... alert ... */ return; }
    try {
      client = await appClientContext.getClient();
      const agentKey: AgentPubKey = client.myPubKey; // Get the raw AgentPubKey

      const playerPayload = { player_key: agentKey, player_name: nickname.trim() };

      const record = await client.callZome({
        cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
        fn_name: "create_player", payload: playerPayload,
      });
      console.log("Player created:", record);

      // FIX: Set profile store with the raw AgentPubKey
      playerProfile.set({
        agentKey: agentKey, // Store the object/Uint8Array
        nickname: nickname.trim()
      });

      dispatch("registered", { nickname: nickname.trim() });
    } catch (e) { /* ... error handling ... */ }
  }
</script>

<div class="popup">
  <h2>Welcome! Let's Pong to Ping!</h2>
  <input type="text" placeholder="Enter your nickname" bind:value={nickname} />
  <button on:click={register}>Register</button>
</div>

<style>
  .popup {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    z-index: 1000;
    color: #fff;
  }
  input {
    margin: 1rem 0;
    padding: 0.5rem;
    font-size: 1rem;
  }
  button {
    padding: 0.5rem 1rem;
    font-size: 1rem;
    border: none;
    border-radius: 4px;
    background: #646cff;
    color: #fff;
    cursor: pointer;
  }
  button:hover {
    background: #535bf2;
  }
</style>
