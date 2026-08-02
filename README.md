# 🏓 Ping2Pong

**Ping2Pong** is a decentralized, peer-to-peer multiplayer Pong arcade game built on **Holochain HDK 0.6** and **Kitsune2 / Iroh P2P networking**. Designed for distribution through **Moss** and the **Holochain Launcher**, it features low-latency physics synchronization, direct signal routing, progressive ball acceleration, hit prediction, and zero central server dependencies.

---

## 🌟 Key Features

- **Decentralized P2P Gameplay**: Powered by Holochain HDK 0.6. No central servers or databases required.
- **Direct Signal Routing (`call_remote`)**: 0ms DHT bypass for real-time physics (`PaddleUpdate`, `BallUpdate`, `PaddleHit`, `ScoreUpdate`) directly between players over WebSockets and global relays.
- **Unrestricted Capability Grants (`pub fn init`)**: Public WASM entrypoint grants global `CapAccess::Unrestricted` for seamless cross-agent signal exchange.
- **Latency & Network Compensation**:
  - **18px Hitbox Grace Margin**: Expands paddle hitboxes vertically by 18px to absorb cross-continent network latency (30–70ms) smoothly.
  - **Authoritative `PaddleHit` Signal**: Instant local deflection feedback ensures defenders are never penalized for last-second saves.
  - **Progressive Ball Acceleration**: Starts with a gentle 2.5 serve velocity and accelerates by 8% on each paddle hit up to a max cap of 12.0.
- **Global Lobby & Online Users**: Real-time presence publishing with online state badges (`READY`, `PLAYING`).
- **Global P2P Chat**: Integrated real-time global chat room.
- **Moss & Holochain Launcher Ready**: Complete with 1024x1024 pixel-art `icon.png` and Moss-compatible `.webhapp` packaging.

---

## 🚀 Quick Start (Development)

### Prerequisites

Ensure you have the [Holochain Development Environment](https://developer.holochain.org/docs/install/) installed.

### Enter Nix Environment & Install Dependencies

```bash
nix develop
npm install
```

### Launch 2-Agent Multi-Window Sandbox

```bash
npm run start
```

This compiles the WASM zomes, packages the `.webhapp` application, and launches 2 connected Holochain conductors (`hc-spin`) with independent UI client windows.

---

## 📦 Building & Packaging for Production / Moss

To package the application into a `.webhapp` distribution file for **Moss** or **Holochain Launcher**:

```bash
npm run package
```

The resulting package will be generated at:
- `workdir/ping2pong.webhapp`

---

## 🛠️ Architecture & Tech Stack

- **Holochain HDK**: `=0.6.1`
- **Holochain HDI**: `=0.7.1`
- **Networking**: Kitsune2 / Iroh WebSockets
- **Frontend**: Svelte + TypeScript + Vite + HTML5 Canvas
- **State Management**: Reactive Svelte Stores (`profilesCache`, `chatStore`, `currentGame`)
