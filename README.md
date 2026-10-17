# Rotating Pyraminx

A small Rust desktop application that renders configurable glTF scenes using Vulkan. The default scene shows seven continuously rotating, four-color Pyraminx models; a Dear ImGui debug overlay provides basic runtime and object information.

## Requirements

- Rust stable with Cargo (Rust 2024 edition support)
- A Vulkan 1.0-compatible graphics device and its installed Vulkan loader/driver

The Vulkan SDK is not needed to build the project; WGSL shaders are compiled to SPIR-V as part of the Cargo build.

## Build and run

From the workspace root:

```powershell
cargo run -p pyraminx-game
```

Cargo builds the game and engine crates and launches the application. The default window is 800 × 600 and is not resizable. The model and shader sources are included in the repository, so no separate asset download is required. The renderer prefers Vulkan's synchronized MAILBOX presentation mode when available and falls back to FIFO otherwise.

## Configure the scene in code

The Pyraminx game setup lives in `game_config()` in `crates/game/src/main.rs`. It uses configuration types and the `run` function exported by the engine crate. A `Scene` is created from its model, instance, camera, and background configuration. Register `set_on_start` and `set_on_update` callbacks for game-specific initialization and per-frame behavior; the update callback receives the frame delta in seconds and mutable instance transforms.

To show another embedded glTF model, add its bytes to `SceneConfig.models` and add an instance whose `model_index` points to that entry:

```rust
let scene = Scene::new(SceneConfig {
    models: vec![
        include_bytes!("../assets/pyraminx.gltf"),
        include_bytes!("../assets/another.gltf"),
    ],
    instances: vec![InstanceConfig {
        name: "Second model".to_owned(),
        model_index: 1,
        position: [0.5, 0.0, 0.0],
        rotation: [0.0; 3],
        scale: [0.5; 3],
    }],
    background_color: [0.03, 0.03, 0.06, 1.0],
    camera: CameraConfig {
        projection: [1.05, 1.4],
        distance: 3.0,
        depth_scale: 1.333333,
    },
});
```

Multiple instances may share a model and independently set their name and transform. Model assignments and scene layout are fixed after startup; update callbacks can change instance names and transforms. Set camera and background settings when creating the scene. The `vulkan-engine` crate handles windowing, scene rendering, picking, and debug UI; a different game can depend on it and provide its own scene, shaders, and assets.

The Pyraminx shader sources are in `crates/game/shaders/` and are compiled to SPIR-V by that crate's build script. Supply shader bytes through `ShaderConfig`; other games can compile and provide their own shaders.

## Controls

- **F3** — Show or hide the debug overlay and hovered-model highlight. The hovered Pyraminx gets a 2-pixel inverted-color border.
- **Right-click a Pyraminx while the overlay is visible** — Open its object-information panel. Right-click elsewhere to dismiss it.
- **Close the window** — Exit the application.

The debug overlay displays the frame rate, uptime, and total scene vertex count. The object panel displays the selected model's rotation direction, transform, vertex and triangle counts, and bounds.

## Tests

Run the unit tests with:

```powershell
cargo test
```

The tests cover scene lifecycle callbacks, loading and validating the embedded Pyraminx mesh, and the renderer's triangle-picking intersection logic.

## Project layout

```text
crates/
  engine/                        Reusable Vulkan engine library
    src/                         Windowing, config, rendering, picking, debug UI
    tests/fixtures/               glTF loader test asset
  game/                          Pyraminx game crate and executable
    src/main.rs                  Game scene setup and executable entry point
    shaders/                     Game WGSL shaders
    build.rs                     Compiles WGSL shaders to SPIR-V
    assets/pyraminx.gltf         Embedded, four-color Pyraminx mesh
```

See [LICENSE](LICENSE) for licensing information.
