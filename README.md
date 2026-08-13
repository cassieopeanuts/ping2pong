# 🏓 Ping2Pong

**Ping2Pong** is a decentralized, peer-to-peer multiplayer Pong arcade game built on **Holochain 0.7** (`HDI 0.8` / `HDK 0.7`) and **Kitsune2 / Iroh P2P networking**.

It was built as a **Proof of Concept (PoC)** to explore and demonstrate the limits of **high-frequency ephemeral signals (`call_remote`)** in Holochain for real-time arcade game physics without central servers or third-party relay infrastructure.

> 💡 **100% Native Holochain Full-Stack**:
> This project uses **pure, unadulterated Holochain**. There are **zero workarounds**, zero central WebSockets servers, zero Firebase/Supabase relay databases, and zero external signaling daemons. All presence, invitations, real-time paddle/ball signals, global chat, player profiles, score validation, and leaderboard rankings run natively through Holochain WASM zomes and Kitsune2 P2P transport.

---

## 📸 Screenshots

### 1. Multi-Agent Lobby & Real-Time P2P Chat
Real-time player presence publishing, instant peer-to-peer game invitations, and global chat over Holochain signals.
![Lobby & Chat](docs/1_lobby_chat.png)

### 2. Real-Time P2P Gameplay
Direct 50fps signal streaming (`PaddleUpdate`, `BallUpdate`, `PaddleHit`) between peer conductors over Kitsune2 WebSockets.
![Live Gameplay](docs/2_gameplay.png)

### 3. Match Finish & Synchronized Game Over Results
Automated victory/defeat modal popups with synchronized final score states delivered to both peers.
![Game Over Modal](docs/3_gameover.png)

### 4. DHT Leaderboard & Persistent Scores
Immutable score creation validated by integrity zomes and aggregated on the P2P DHT.
![Post-Game Leaderboard](docs/4_leaderboard.png)

---

## 🌟 Key Technical Features

- **100% Decentralized P2P Stack**: Built on Holochain 0.7 (`hdi = "=0.8.0"`, `hdk = "=0.7.0"`). No central servers or databases required.
- **Direct Ephemeral Signals (`call_remote`)**: Bypasses DHT storage for high-frequency real-time physics (`PaddleUpdate`, `BallUpdate`, `PaddleHit`, `ScoreUpdate`, `GameOver`) directly between peer conductors.
- **Unrestricted Capability Grants (`pub fn init`)**: Public WASM entrypoint grants global `CapAccess::Unrestricted` for seamless cross-agent signal exchange.
- **50fps High-Frequency Sync & Lag Compensation**:
  - **20ms Signal Interval**: Transmits 50 updates per second during active motion tracking.
  - **Trailing Stop Sync**: Guaranteed final-position signal delivery on key release so paddle positions remain locked across peers.
  - **18px Hitbox Grace Margin**: Expands paddle hitboxes vertically by 18px to absorb cross-continent P2P latency smoothly.
  - **Authoritative `PaddleHit` Signal**: Instant local deflection feedback ensures defenders are never penalized for last-second saves.
- **Global Lobby & Presence**: Real-time presence publishing with online state badges (`READY`, `PLAYING`).
- **Global P2P Chat**: Integrated real-time global chat room powered by ephemeral signals.
- **Moss & Holochain Launcher Ready**: Includes a 1024x1024 pixel-art `icon.png` and standard Moss `.webhapp` packaging.

---

## 🚀 Quick Start (Development & Testing)

### 1. Prerequisites & Environment Setup

#### Native Linux / macOS
Ensure you have Nix installed with Flakes enabled:
```bash
# Install Nix (Multi-user recommended)
curl -L https://nixos.org/nix/install | sh

# Enable Flakes in ~/.config/nix/nix.conf
experimental-features = nix-command flakes
```

#### Windows via WSL2 (Windows Subsystem for Linux)
For Windows developers, we recommend using **WSL2 (Ubuntu 22.04 / 24.04)**:

1. **Install WSL2 & Ubuntu**:
   ```powershell
   wsl --install -d Ubuntu
   ```
2. **Enable Systemd in WSL2**:
   Edit `/etc/wsl.conf` inside your WSL terminal:
   ```ini
   [boot]
   systemd=true
   ```
   Then restart WSL from Windows PowerShell: `wsl --shutdown`.
3. **Install Nix inside WSL2**:
   ```bash
   curl -L https://nixos.org/nix/install | sh --daemon
   ```
4. **IMPORTANT - Use Native Linux Filesystem**:
   Clone and build the repository inside your Linux home directory (e.g. `/home/username/code/ping2pong`). **Do not** run from `/mnt/c/`, as Windows filesystem mounting degrades Rust compile speeds and file watcher events.
5. **GUI Display (WSLg)**:
   Windows 11 supports WSLg out of the box. `hc-spin` will open the UI electron/browser windows directly on your Windows desktop!

---

### 2. Enter Nix Environment & Install Dependencies

Enter the isolated Holochain 0.7 development shell. This automatically provisions the exact versions of Rust, `wasm32-unknown-unknown` target, `holochain` binary (`0.4`), `hc` CLI, `hc-spin`, and Node.js:

```bash
# Clone the repository
git clone https://github.com/cassieopeanuts/ping2pong.git
cd ping2pong

# Enter the Holochain Nix development shell
nix develop

# Install Node.js workspace dependencies
npm install
```

---

### 3. Launching Multi-Agent Sandbox Sessions

#### Launch Default 2-Agent Multi-Window Sandbox
```bash
npm run start
```
This command:
1. Compiles the WASM Integrity and Coordinator zomes (`npm run build:zomes`).
2. Bundles the UI and packages `workdir/ping2pong.webhapp`.
3. Spins up **2 separate Holochain conductors** (`hc-spin`) in temporary sandboxes.
4. Opens two independent UI client windows side-by-side so you can register two players (`Player 1` and `Player 2`), send invitations, chat, and test gameplay in real-time.

#### Test with 3+ Agents
Want to test global chat or multi-user lobbies with 3 or 4 players?
```bash
AGENTS=3 npm run start
```

---

## 📦 Building & Packaging for Production / Moss

To package the application into a `.webhapp` distribution file for **Moss** or **Holochain Launcher**:

```bash
npm run package
```

The resulting package will be generated at:
- `workdir/ping2pong.webhapp`

---

## 🛠️ Developer & Contributor Guide

### Workspace Structure

```
ping2pong/
├── dnas/
│   └── ping_2_pong/
│       └── zomes/
│           ├── integrity/ping_2_pong/   # WASM Integrity Zome (Entry types & DHT validation)
│           └── coordinator/ping_2_pong/ # WASM Coordinator Zome (Signals, presence, chat, score logic)
├── ui/                                   # Svelte + Vite + HTML5 Canvas Frontend
│   ├── src/
│   │   ├── ping_2_pong/
│   │   │   ├── game/                   # PongGame.svelte, Leaderboard.svelte, Lobby.svelte
│   │   │   └── chat/                   # GlobalChat.svelte
│   │   └── stores/                     # Svelte reactive stores (profiles, chat, game state)
│   └── public/
├── workdir/                              # Holochain app bundles (.happ, .dna, .webhapp)
├── flake.nix                             # Holochain 0.7 Nix Flake development shell definition
└── README.md
```

### Helpful Development Commands

| Command | Action |
| :--- | :--- |
| `nix develop` | Enters the reproducible Holochain 0.7 shell environment |
| `npm run start` | Builds zomes, packages `.webhapp`, and spins 2 sandboxed agents |
| `AGENTS=3 npm run start` | Spins 3 connected sandboxed agents for multi-user lobby testing |
| `npm run build:zomes` | Compiles Rust zomes to `wasm32-unknown-unknown` without launching UI |
| `npm run package` | Builds release zomes and packages `workdir/ping2pong.webhapp` |
| `npm run dev --workspace ui` | Runs UI Vite dev server only with hot module reloading (HMR) |

---

## 🔒 Security & Data Integrity

- **Cryptographic Signatures**: Every action on Holochain is cryptographically signed by the agent's private key (`SignedActionHashed`). Impersonation is mathematically impossible.
- **Deterministic Name Anchors**: Nicknames are tied to deterministic DHT anchor hashes (`Path::from("nickname").path_entry_hash()`). Duplicate name registrations are blocked at the zome level.
- **Validation Rules**: `score_validation.rs` verifies that scores can only be recorded for games that exist on the DHT and that the scorer was a verified participant.

---

## 🛠️ Architecture & Tech Stack

- **Holochain HDK**: `=0.7.0`
- **Holochain HDI**: `=0.8.0`
- **Networking**: Kitsune2 / Iroh WebSockets
- **Frontend**: Svelte + TypeScript + Vite + HTML5 Canvas
- **State Management**: Reactive Svelte Stores (`profilesCache`, `chatStore`, `currentGame`)
