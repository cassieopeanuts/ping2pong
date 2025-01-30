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

import { createScore, sampleScore } from "./common.js";

test("create Score", async () => {
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

    // Alice creates a Score
    const record: Record = await createScore(alice.cells[0]);
    assert.ok(record);
  });
});

test("create and read Score", async () => {
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

    const sample = await sampleScore(alice.cells[0]);

    // Alice creates a Score
    const record: Record = await createScore(alice.cells[0], sample);
    assert.ok(record);

    // Wait for the created entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the created Score
    const createReadOutput: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_original_score",
      payload: record.signed_action.hashed.hash,
    });
    assert.deepEqual(sample, decode((createReadOutput.entry as any).Present.entry) as any);

    // Bob gets the Players for the new Score
    let linksToPlayers: Link[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_scores_for_player",
      payload: sample.player,
    });
    assert.equal(linksToPlayers.length, 1);
    assert.deepEqual(linksToPlayers[0].target, record.signed_action.hashed.hash);
  });
});

test("create and update Score", async () => {
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

    // Alice creates a Score
    const record: Record = await createScore(alice.cells[0]);
    assert.ok(record);

    const originalActionHash = record.signed_action.hashed.hash;

    // Alice updates the Score
    let contentUpdate: any = await sampleScore(alice.cells[0]);
    let updateInput = {
      original_score_hash: originalActionHash,
      previous_score_hash: originalActionHash,
      updated_score: contentUpdate,
    };

    let updatedRecord: Record = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "update_score",
      payload: updateInput,
    });
    assert.ok(updatedRecord);

    // Wait for the updated entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the updated Score
    const readUpdatedOutput0: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_latest_score",
      payload: updatedRecord.signed_action.hashed.hash,
    });
    assert.deepEqual(contentUpdate, decode((readUpdatedOutput0.entry as any).Present.entry) as any);

    // Alice updates the Score again
    contentUpdate = await sampleScore(alice.cells[0]);
    updateInput = {
      original_score_hash: originalActionHash,
      previous_score_hash: updatedRecord.signed_action.hashed.hash,
      updated_score: contentUpdate,
    };

    updatedRecord = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "update_score",
      payload: updateInput,
    });
    assert.ok(updatedRecord);

    // Wait for the updated entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the updated Score
    const readUpdatedOutput1: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_latest_score",
      payload: updatedRecord.signed_action.hashed.hash,
    });
    assert.deepEqual(contentUpdate, decode((readUpdatedOutput1.entry as any).Present.entry) as any);

    // Bob gets all the revisions for Score
    const revisions: Record[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_all_revisions_for_score",
      payload: originalActionHash,
    });
    assert.equal(revisions.length, 3);
    assert.deepEqual(contentUpdate, decode((revisions[2].entry as any).Present.entry) as any);
  });
});

test("create and delete Score", async () => {
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

    const sample = await sampleScore(alice.cells[0]);

    // Alice creates a Score
    const record: Record = await createScore(alice.cells[0], sample);
    assert.ok(record);

    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the Players for the new Score
    let linksToPlayers: Link[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_scores_for_player",
      payload: sample.player,
    });
    assert.equal(linksToPlayers.length, 1);
    assert.deepEqual(linksToPlayers[0].target, record.signed_action.hashed.hash);

    // Alice deletes the Score
    const deleteActionHash = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "delete_score",
      payload: record.signed_action.hashed.hash,
    });
    assert.ok(deleteActionHash);

    // Wait for the entry deletion to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the oldest delete for the Score
    const oldestDeleteForScore: SignedActionHashed = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_oldest_delete_for_score",
      payload: record.signed_action.hashed.hash,
    });
    assert.ok(oldestDeleteForScore);

    // Bob gets the deletions for the Score
    const deletesForScore: SignedActionHashed[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_all_deletes_for_score",
      payload: record.signed_action.hashed.hash,
    });
    assert.equal(deletesForScore.length, 1);

    // Bob gets the Players for the Score again
    linksToPlayers = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_scores_for_player",
      payload: sample.player,
    });
    assert.equal(linksToPlayers.length, 0);

    // Bob gets the deleted Players for the Score
    const deletedLinksToPlayers: Array<[SignedActionHashed<CreateLink>, SignedActionHashed<DeleteLink>[]]> = await bob
      .cells[0].callZome({
        zome_name: "ping_2_pong",
        fn_name: "get_deleted_scores_for_player",
        payload: sample.player,
      });
    assert.equal(deletedLinksToPlayers.length, 1);
  });
});
