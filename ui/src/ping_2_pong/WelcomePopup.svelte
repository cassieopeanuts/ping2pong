<script lang="ts">
  import { createEventDispatcher, getContext } from "svelte";
  import { playerProfile } from "../stores/playerProfile";
  // Import the client context and its type.
  import { clientContext, type ClientContext } from "../contexts";
  import type { AppClient, AgentPubKey } from "@holochain/client";

  const dispatch = createEventDispatcher();
  let nickname: string = "";
  let client: AppClient;

  // Explicitly tell TypeScript the type of the client context.
  const appClientContext = getContext<ClientContext>(clientContext);

  async function register() {
    if (nickname.trim() === "") {
      alert("Please enter a nickname.");
      return;
    }
    try {
      // Get the Holochain client.
      client = await appClientContext.getClient();

      // Retrieve the agent's public key.
      // (Assuming client.myPubKey returns a HoloHash/ActionHash.)
      const agentKey: AgentPubKey = client.myPubKey;

      // Build the player object required by your DNA.
      const player = {
        player_key: agentKey,
        player_name: nickname.trim()
      };

      // Call the DNA function "create_player".
      const record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_player",
        payload: player,
      });

      console.log("Player created:", record);

      // Convert the agent key to a string before storing.
      playerProfile.set({ agentKey: agentKey.toString(), nickname: nickname.trim() });

      dispatch("registered", { nickname: nickname.trim() });
    } catch (e) {
      console.error("Error registering player:", e);
      alert("Error registering player: " + (e as Error).message);
    }
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
