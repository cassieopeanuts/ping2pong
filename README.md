# ping2pong

**ping2pong** is a modern implementation of the classic arcade game Pong, rebuilt with a decentralized architecture powered by Holochain (0.4.0v). It leverages Holochain's capabilities to manage game state and player interactions in a peer-to-peer manner, offering a unique take on familiar gameplay. This project showcases how Holochain can be used for real-time applications, utilizing ephemeral signals for propagating ball and paddle positions between players.

## Project Overview and Special Features

**ping2pong** uses **Holochain's ephemeral signals** for managing real-time game events. This allows for low-latency, peer-to-peer synchronization of crucial game elements like ball and paddle positions directly between players, ensuring smooth and responsive gameplay.

Key features of **ping2pong** include:

*   **Decentralized Gameplay:** Built on Holochain, all game logic and data are managed in a distributed manner.
*   **Real-time P2P Sync:** Utilizes Holochain signals for instant updates of game state between players.
*   **Invitation System:** Challenge other players to a match through an intuitive invitation system.
*   **Global Chat:** Communicate with other players in a global chat room.

## Environment Setup

> PREREQUISITE: set up the [holochain development environment](https://developer.holochain.org/docs/install/).

Enter the nix shell by running this in the root folder of the repository: 

```bash
nix develop
npm install
```

**Run all the other instructions in this README from inside this nix shell, otherwise they won't work**.

## Running 2 agents
 
```bash
npm run start
```

This will create a network of 2 nodes connected to each other and their respective UIs.
It will also bring up the Holochain Playground for advanced introspection of the conductors.

## Running the backend tests

```bash
npm run test
```

## Bootstrapping a network

Create a custom network of nodes connected to each other and their respective UIs with:

```bash
AGENTS=3 npm run network
```

Substitute the "3" for the number of nodes that you want to bootstrap in your network.
This will also bring up the Holochain Playground for advanced introspection of the conductors.

## Packaging

To package the web happ:
``` bash
npm run build:happ

npm run package
```

You'll have the `ping2pong.webhapp` in `workdir`. This is what you should distribute so that the Holochain Launcher can install it.
You will also have its subcomponent `ping2pong.happ` in the same folder`.

## OR download already Packaged Application (`.webhapp`) and install in  ([Holochain Launcher](https://github.com/holochain/launcher))

For convenience, a pre-packaged `ping2pong.webhapp` file is included in this repository. You can download it directly from the following location:

-   [./workdir/ping2pong.webhapp](./workdir/ping2pong.zip)

## Documentation

This repository is using these tools:
- [NPM Workspaces](https://docs.npmjs.com/cli/v7/using-npm/workspaces/): npm v7's built-in monorepo capabilities.
- [hc](https://github.com/holochain/holochain/tree/develop/crates/hc): Holochain CLI to easily manage Holochain development instances.
- [@holochain/tryorama](https://www.npmjs.com/package/@holochain/tryorama): test framework.
- [@holochain/client](https://www.npmjs.com/package/@holochain/client): client library to connect to Holochain from the UI.
- [@holochain-playground/cli](https://www.npmjs.com/package/@holochain-playground/cli): introspection tooling to understand what's going on in the Holochain nodes.
