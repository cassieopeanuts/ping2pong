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

import { createStatistics, sampleStatistics } from "./common.js";

test("create Statistics", async () => {
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

    // Alice creates a Statistics
    const record: Record = await createStatistics(alice.cells[0]);
    assert.ok(record);
  });
});

test("create and read Statistics", async () => {
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

    const sample = await sampleStatistics(alice.cells[0]);

    // Alice creates a Statistics
    const record: Record = await createStatistics(alice.cells[0], sample);
    assert.ok(record);

    // Wait for the created entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the created Statistics
    const createReadOutput: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_original_statistics",
      payload: record.signed_action.hashed.hash,
    });
    assert.deepEqual(sample, decode((createReadOutput.entry as any).Present.entry) as any);
  });
});

test("create and update Statistics", async () => {
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

    // Alice creates a Statistics
    const record: Record = await createStatistics(alice.cells[0]);
    assert.ok(record);

    const originalActionHash = record.signed_action.hashed.hash;

    // Alice updates the Statistics
    let contentUpdate: any = await sampleStatistics(alice.cells[0]);
    let updateInput = {
      original_statistics_hash: originalActionHash,
      previous_statistics_hash: originalActionHash,
      updated_statistics: contentUpdate,
    };

    let updatedRecord: Record = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "update_statistics",
      payload: updateInput,
    });
    assert.ok(updatedRecord);

    // Wait for the updated entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the updated Statistics
    const readUpdatedOutput0: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_latest_statistics",
      payload: updatedRecord.signed_action.hashed.hash,
    });
    assert.deepEqual(contentUpdate, decode((readUpdatedOutput0.entry as any).Present.entry) as any);

    // Alice updates the Statistics again
    contentUpdate = await sampleStatistics(alice.cells[0]);
    updateInput = {
      original_statistics_hash: originalActionHash,
      previous_statistics_hash: updatedRecord.signed_action.hashed.hash,
      updated_statistics: contentUpdate,
    };

    updatedRecord = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "update_statistics",
      payload: updateInput,
    });
    assert.ok(updatedRecord);

    // Wait for the updated entry to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the updated Statistics
    const readUpdatedOutput1: Record = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_latest_statistics",
      payload: updatedRecord.signed_action.hashed.hash,
    });
    assert.deepEqual(contentUpdate, decode((readUpdatedOutput1.entry as any).Present.entry) as any);

    // Bob gets all the revisions for Statistics
    const revisions: Record[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_all_revisions_for_statistics",
      payload: originalActionHash,
    });
    assert.equal(revisions.length, 3);
    assert.deepEqual(contentUpdate, decode((revisions[2].entry as any).Present.entry) as any);
  });
});

test("create and delete Statistics", async () => {
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

    const sample = await sampleStatistics(alice.cells[0]);

    // Alice creates a Statistics
    const record: Record = await createStatistics(alice.cells[0], sample);
    assert.ok(record);

    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Alice deletes the Statistics
    const deleteActionHash = await alice.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "delete_statistics",
      payload: record.signed_action.hashed.hash,
    });
    assert.ok(deleteActionHash);

    // Wait for the entry deletion to be propagated to the other node.
    await dhtSync([alice, bob], alice.cells[0].cell_id[0]);

    // Bob gets the oldest delete for the Statistics
    const oldestDeleteForStatistics: SignedActionHashed = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_oldest_delete_for_statistics",
      payload: record.signed_action.hashed.hash,
    });
    assert.ok(oldestDeleteForStatistics);

    // Bob gets the deletions for the Statistics
    const deletesForStatistics: SignedActionHashed[] = await bob.cells[0].callZome({
      zome_name: "ping_2_pong",
      fn_name: "get_all_deletes_for_statistics",
      payload: record.signed_action.hashed.hash,
    });
    assert.equal(deletesForStatistics.length, 1);
  });
});
