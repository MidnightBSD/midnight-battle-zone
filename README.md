# Midnight Battle Zone

A cross-platform 3D real-time strategy (RTS) game built in **Rust** using **Vulkan** (`ash` + `naga`), **Lua 5.4** scripting, and **SDL3** gamepad support, designed to build and run cleanly on **Linux**, **FreeBSD**, and **MidnightBSD** (as well as Steam / Steam Deck).

---

## Features

- **3D Vulkan Isometric Renderer**: Procedural 3D low-poly **Cat** units (**Worker Cat**, **Soldier Cat**, **Sniper Cat**) and base structures (**Command Center**, **House**, **Resource Mine**, **Barracks**), terrain, crystal mineral nodes, selection rings, interactive minimap, and real-time **Fog of War** shader.
- **AMD FidelityFX Super Resolution (FSR 1.0)**: Built-in EASU (Edge-Adaptive Spatial Upsampling) + RCAS (Robust Contrast-Adaptive Sharpening) pass with `Off`, `Ultra Quality (1.3x)`, `Quality (1.5x)`, `Balanced (1.7x)`, and `Performance (2.0x)` presets (`[F]` key).
- **SDL3 Gamepad & Steam Deck Support**: Full controller support via `libSDL3` (`SDL_Gamepad` API) for cursor control, box-selection, camera panning/zooming, building construction, and unit commands.
- **Top-Right FPS & Time Counter**: Live `FPS` and `TIME (MM:SS)` overlay in the top-right corner of the window.
- **Synchronized Two-Player Pause**: Press `[P]` (or `Start` on a gamepad) or click `PAUSE [P]` in the top bar to pause/resume the game for **both players** in Singleplayer or LAN multiplayer.
- **Cat Meow Task Acknowledgments**: Assigning tasks to **Worker Cats** or **Soldier Cats** triggers synthesized cat meows (`"Meow!"` / `"Mrow!"`) and floating on-screen speech callouts.
- **Optional Public-Domain Jazz & Blues Background Music**: Cycle between `Off`, *St. Louis Blues (1914 PD Blues)*, *Midnight Cool Jazz (PD Swing)*, and *12-Bar Blues Shuffle (PD Blues)* anytime with `[J]` or load a custom `.wav` file with `--music <path>`.
- **Three Battle Maps (3–6 Mines per Side)**:
  - **Midnight Valley** (`midnight_valley.lua`): 4 resource nodes per side (8 total)
  - **Twin Iron Ridge** (`twin_iron_ridge.lua`): 3 resource nodes per side (6 total)
  - **Shattered Basin** (`shattered_basin.lua`): 6 resource nodes per side (12 total)
- **Three Lua AI Difficulty Levels**: `Easy`, `Medium`, and `Hard` scripted in `assets/scripts/ai.lua`.
- **Two-Player LAN Multiplayer**: Deterministic 20 Hz lockstep UDP networking with **Multicast DNS (mDNS)** auto-discovery (`224.0.0.251:5353`, `_mbz-rts._udp.local.`) and **Direct IP:Port** connection.

---

## Build & Runtime Dependencies

Because **Midnight Battle Zone** compiles **Lua 5.4** from vendored ANSI C source (`mlua`) and compiles **WGSL shaders to SPIR-V** in pure Rust (`naga`), you do **not** need external `glslc`, `shaderc`, or `liblua-dev` packages to build it.

### Required Dependencies
1. **Rust Toolchain** (`rustc >= 1.75`, `cargo`)
2. **C Compiler & Linker** (`cc` / `gcc` / `clang` and `pkg-config`) — used by Cargo to compile the bundled Lua 5.4 library
3. **X11 Development Library** (`libX11`) — linked for window creation
4. **Vulkan Loader & GPU Driver** (`libvulkan.so.1` / `vulkan-loader` + Mesa/proprietary ICD) — dynamically loaded at runtime

### Optional Runtime Dependencies
5. **SDL3 Shared Library** (`libSDL3.so.0` / `libSDL3.so`) — automatically detected and loaded at runtime for **Gamepad / Steam Controller / Steam Deck** support
6. **System Audio Player** (`pw-play`, `paplay`, `aplay` on Linux; `paplay` or `aucat` on FreeBSD/MidnightBSD) — used for non-blocking Cat Meow sound effects and Jazz/Blues background music

---

### Installing Dependencies by Operating System

#### Ubuntu / Debian Linux
```bash
sudo apt update
sudo apt install build-essential pkg-config libx11-dev libvulkan1 mesa-vulkan-drivers libsdl3-0 pulseaudio-utils
```
*(If Rust is not installed yet: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)*

#### Fedora / RHEL Linux
```bash
sudo dnf install gcc make pkgconf-pkg-config libX11-devel vulkan-loader mesa-vulkan-drivers SDL3 pulseaudio-utils
```

#### FreeBSD
```bash
sudo pkg install rust pkgconf libX11 vulkan-loader mesa-dri sdl3 pulseaudio
```

#### MidnightBSD
```bash
sudo mport install rust pkgconf libX11 vulkan-loader mesa-dri sdl3
```
> **Note for MidnightBSD**: `build.rs` automatically adds `/usr/local/lib` to the native linker search path for both `freebsd` and `midnightbsd` target configurations. If building with an upstream Rust toolchain on MidnightBSD, target `x86_64-unknown-freebsd`.

---

## Building and Installing the Game

### 1. Clone and Build Release Binary
```bash
git clone <repo-url> midnight-battle-zone
cd midnight-battle-zone
cargo build --release
```

### 2. Run the Automated Test Suite
```bash
cargo test
```

### 3. Install Locally (Optional)
To install the binary and assets into `/usr/local/share/midnight-battle-zone` (or `~/.local/share/midnight-battle-zone`):
```bash
mkdir -p ~/.local/share/midnight-battle-zone ~/.local/bin
cp target/release/midnight-battle-zone ~/.local/share/midnight-battle-zone/
cp -r assets ~/.local/share/midnight-battle-zone/
cat << 'EOF' > ~/.local/bin/midnight-battle-zone
#!/bin/sh
cd "$HOME/.local/share/midnight-battle-zone" && exec ./midnight-battle-zone "$@"
EOF
chmod +x ~/.local/bin/midnight-battle-zone
```

---

## Running the Game & CLI Options

```bash
# Launch into the interactive Main Menu
cargo run --release

# Start immediately on Shattered Basin against Hard AI with AMD FSR Quality and Cool Jazz music
cargo run --release -- --map shattered_basin --ai hard --fsr quality --music jazz

# Host a 2-player LAN game on port 28765 (advertised via mDNS)
cargo run --release -- --host 28765 --map midnight_valley

# Connect to a LAN host by direct IP:Port
cargo run --release -- --connect 192.168.1.42:28765

# Run a headless deterministic simulation (for testing/CI)
cargo run --release -- --headless --ticks 300 --map twin_iron_ridge --ai hard
```

---

## Controls (Keyboard, Mouse & SDL3 Gamepad)

| Action | Keyboard & Mouse | SDL3 Gamepad / Steam Deck |
|---|---|---|
| **Move Cursor** | Mouse Move | **Left Stick** |
| **Select / Box-Select / Click UI** | **Left Click** (Hold & Drag to box-select) | **South Button (`A` / Cross)** (Hold & Move Left Stick to box-select) |
| **Context Order (Move / Attack / Mine / Repair)** | **Right Click** | **East Button (`B` / Circle)** |
| **Pan 3D Camera** | **Arrow Keys** | **Right Stick** |
| **Zoom 3D Camera** | **Mouse Scroll Wheel** | **Left / Right Shoulders (`LB` / `RB`)** or **Triggers (`LT` / `RT`)** |
| **Train Worker Cat / Soldier Cat / Attack-Move** | **`[W]`** / **`[S]`** / **`[A]`** | **West Button (`X` / Square)** |
| **Build Mine / Train Sniper Cat** | **`[M]`** / **`[N]`** | **North Button (`Y` / Triangle)** |
| **Build House** | **`[H]`** | **D-Pad Up** |
| **Build Barracks** | **`[B]`** | **D-Pad Right** |
| **Repair Building** | **`[R]`** | **D-Pad Down** |
| **Stop Selected Units** | **`[X]`** | **D-Pad Left** |
| **Pause / Resume (Both Players)** | **`[P]`** | **Start Button** |
| **Cycle AMD FSR 1.0 Mode** | **`[F]`** | **Back / Select Button** |
| **Cycle Public-Domain Jazz & Blues Music** | **`[J]`** | **Guide / Home Button** |

---

## Adding Custom Units in Lua

All units and buildings are data-driven via [assets/scripts/units.lua](file:///home/laffer1/git/midnight-battle-zone/assets/scripts/units.lua) and [assets/scripts/buildings.lua](file:///home/laffer1/git/midnight-battle-zone/assets/scripts/buildings.lua). To add a new unit type, append a table entry to `assets/scripts/units.lua` and add its `id` to `trainable_units` in `assets/scripts/buildings.lua`.
