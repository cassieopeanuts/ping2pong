import { assert, test } from "vitest";

import {
  ActionHash,
  AppBundleSource,
  CreateLink,
  DeleteLink,
  fakeActionHash,
  fakeAgentPubKey,
  fakeEntryHash,
  Link,
  NewEntryAction,
  Record,
  SignedActionHashed,
} from "@holochain/client";
import { CallableCell, dhtSync, runScenario } from "@holochain/tryorama";
import { decode } from "@msgpack/msgpack";

import { createGame, sampleGame } from "./common.js";

test("create Game", async () => {
  await runScenario(async scenario => {
    // Construct proper paths for your app.
    // This assumes app bundle created by the `hc app pack` command.
    const testAppPath = process.cwd() + "/../workdir/ping2pong.happ";

    // Set up the app to be installed
    const appSource = { appBundleSource: { path: testAppPath } };

    // Add 2 players with the test app to the Scenario. The returned players
    // can be destructured.
    const [alice, bob] = await scenario.addPlayersWithApps([appSource, appSource]);

    // Shortcut peer discovery through gossip and register all agents in every
    // conductor of the scenario.
    await scenario.shareAllAgents();

    // Alice creates a Game
    const record: Record = await createGame(alice.cells[0]);
    assert.ok(record);
  });
});

test("create and read Game", async () => {
  await runScenario(async scenario => {
    // Construct proper paths for your app.
    // This assumes app bundle created by the `hc app pack` command.
    const testAppPath = process.cwd() + "/../workdir/ping2pong.happ";

    // Set up the app to be installed
    const appSource = { appBundleSource: { path: testAppPath } };

    // Add 2 players with the test app to the Scenario. The returned players
    // can be destructured.
    const [alice, bob] = await scenario.addPlayersWithApps([appSource, appSource]);

    // Shortcut peer discovery through gossip and register all agents in every
    // conductor of the scenario.
    await scenario.shareAllAgents();

    const sample = await sampleGame(alice.cells[0]);

    // Alice creates a Game
    const record: Record = await createGame(alice.cells[0], sample);
    assert.ok(record);

    // Wait for the created entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the created Game
    const createReadOutput: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_original_game",
      payload: record.signed_action.hashed.hash,
    });
    assert.deepEqual(sample, decode((createReadOutput.entry as any).Present.entry) as any);

    // Bob gets the Player1S for the new Game
    let linksToPlayer1S: Link[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_games_for_player_1",
      payload: sample.player_1,
    });
    assert.equal(linksToPlayer1S.length, 1);
    assert.deepEqual(linksToPlayer1S[0].target, record.signed_action.hashed.hash);
    // Bob gets the Player2S for the new Game
    let linksToPlayer2S: Link[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_games_for_player_2",
      payload: sample.player_2,
    });
    assert.equal(linksToPlayer2S.length, 1);
    assert.deepEqual(linksToPlayer2S[0].target, record.signed_action.hashed.hash);
  });
});

test("create and update Game", async () => {
  await runScenario(async scenario => {
    // Construct proper paths for your app.
    // This assumes app bundle created by the `hc app pack` command.
    const testAppPath = process.cwd() + "/../workdir/ping2pong.happ";

    // Set up the app to be installed
    const appSource = { appBundleSource: { path: testAppPath } };

    // Add 2 players with the test app to the Scenario. The returned players
    // can be destructured.
    const [alice, bob] = await scenario.addPlayersWithApps([appSource, appSource]);

    // Shortcut peer discovery through gossip and register all agents in every
    // conductor of the scenario.
    await scenario.shareAllAgents();

    // Alice creates a Game
    const record: Record = await createGame(alice.cells[0]);
    assert.ok(record);

    const originalActionHash = record.signed_action.hashed.hash;

    // Alice updates the Game
    let contentUpdate: any = await sampleGame(alice.cells[0]);
    let updateInput = {
      original_game_hash: originalActionHash,
      previous_game_hash: originalActionHash,
      updated_game: contentUpdate,
    };

    let updatedRecord: Record = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "update_game",
      payload: updateInput,
    });
    assert.ok(updatedRecord);

    // Wait for the updated entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the updated Game
    const readUpdatedOutput0: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_latest_game",
      payload: updatedRecord.signed_action.hashed.hash,
    });
    assert.deepEqual(contentUpdate, decode((readUpdatedOutput0.entry as any).Present.entry) as any);

    // Alice updates the Game again
    contentUpdate = await sampleGame(alice.cells[0]);
    updateInput = {
      original_game_hash: originalActionHash,
      previous_game_hash: updatedRecord.signed_action.hashed.hash,
      updated_game: contentUpdate,
    };

    updatedRecord = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "update_game",
      payload: updateInput,
    });
    assert.ok(updatedRecord);

    // Wait for the updated entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the updated Game
    const readUpdatedOutput1: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_latest_game",
      payload: updatedRecord.signed_action.hashed.hash,
    });
    assert.deepEqual(contentUpdate, decode((readUpdatedOutput1.entry as any).Present.entry) as any);

    // Bob gets all the revisions for Game
    const revisions: Record[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_all_revisions_for_game",
      payload: originalActionHash,
    });
    assert.equal(revisions.length, 3);
    assert.deepEqual(contentUpdate, decode((revisions[2].entry as any).Present.entry) as any);
  });
});

test("create and delete Game", async () => {
  await runScenario(async scenario => {
    // Construct proper paths for your app.
    // This assumes app bundle created by the `hc app pack` command.
    const testAppPath = process.cwd() + "/../workdir/ping2pong.happ";

    // Set up the app to be installed
    const appSource = { appBundleSource: { path: testAppPath } };

    // Add 2 players with the test app to the Scenario. The returned players
    // can be destructured.
    const [alice, bob] = await scenario.addPlayersWithApps([appSource, appSource]);

    // Shortcut peer discovery through gossip and register all agents in every
    // conductor of the scenario.
    await scenario.shareAllAgents();

    const sample = await sampleGame(alice.cells[0]);

    // Alice creates a Game
    const record: Record = await createGame(alice.cells[0], sample);
    assert.ok(record);

    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the Player1S for the new Game
    let linksToPlayer1S: Link[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_games_for_player_1",
      payload: sample.player_1,
    });
    assert.equal(linksToPlayer1S.length, 1);
    assert.deepEqual(linksToPlayer1S[0].target, record.signed_action.hashed.hash);
    // Bob gets the Player2S for the new Game
    let linksToPlayer2S: Link[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_games_for_player_2",
      payload: sample.player_2,
    });
    assert.equal(linksToPlayer2S.length, 1);
    assert.deepEqual(linksToPlayer2S[0].target, record.signed_action.hashed.hash);

    // Alice deletes the Game
    const deleteActionHash = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "delete_game",
      payload: record.signed_action.hashed.hash,
    });
    assert.ok(deleteActionHash);

    // Wait for the entry deletion to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the oldest delete for the Game
    const oldestDeleteForGame: SignedActionHashed = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_oldest_delete_for_game",
      payload: record.signed_action.hashed.hash,
    });
    assert.ok(oldestDeleteForGame);

    // Bob gets the deletions for the Game
    const deletesForGame: SignedActionHashed[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_all_deletes_for_game",
      payload: record.signed_action.hashed.hash,
    });
    assert.equal(deletesForGame.length, 1);

    // Bob gets the Player1S for the Game again
    linksToPlayer1S = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_games_for_player_1",
      payload: sample.player_1,
    });
    assert.equal(linksToPlayer1S.length, 0);

    // Bob gets the deleted Player1S for the Game
    const deletedLinksToPlayer1S: Array<[SignedActionHashed<CreateLink>, SignedActionHashed<DeleteLink>[]]> = await bob
      .cells[0].callZome({
        zome_name: "ping_2_pong",
        fn_name: "get_deleted_games_for_player_1",
        payload: sample.player_1,
      });
    assert.equal(deletedLinksToPlayer1S.length, 1);
    // Bob gets the Player2S for the Game again
    linksToPlayer2S = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_games_for_player_2",
      payload: sample.player_2,
    });
    assert.equal(linksToPlayer2S.length, 0);

    // Bob gets the deleted Player2S for the Game
    const deletedLinksToPlayer2S: Array<[SignedActionHashed<CreateLink>, SignedActionHashed<DeleteLink>[]]> = await bob
      .cells[0].callZome({
        zome_name: "ping_2_pong",
        fn_name: "get_deleted_games_for_player_2",
        payload: sample.player_2,
      });
    assert.equal(deletedLinksToPlayer2S.length, 1);
  });
});
