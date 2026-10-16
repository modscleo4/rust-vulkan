# Rotating Pyraminx

A small Rust desktop application that renders configurable glTF scenes using Vulkan. The default scene shows seven continuously rotating, four-color Pyraminx models; a Dear ImGui debug overlay provides basic runtime and object information.

## Requirements

- Rust stable with Cargo (Rust 2024 edition support)
- A Vulkan 1.0-compatible graphics device and its installed Vulkan loader/driver

The Vulkan SDK is not needed to build the project; WGSL shaders are compiled to SPIR-V as part of the Cargo build.

## Build and run

From the project directory:

```powershell
cargo run
```

Cargo builds the application and launches it. The default window is 800 × 600 and is not resizable. The model and shader sources are included in the repository, so no separate asset download is required.

## Configure the scene in code

The example setup lives in `AppConfig::default()` in `src/config.rs`. `src/main.rs` passes that configuration to the app, so you can adjust the window, embedded model assets, instances, camera, animation speed, and background color there. For example, replace the body of `main` with:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = config::AppConfig::default();
    config.window.title = "My Vulkan Scene".to_owned();
    config.window.size = [1280, 720];
    config.window.resizable = true;
    config.scene.animation_speed = 1.2;
    config.scene.background_color = [0.02, 0.04, 0.08, 1.0];
    config.scene.instances.truncate(1);
    config.scene.instances[0].name = "Example".to_owned();
    config.scene.instances[0].animation_axes = [false, true, false];
    app::run(config)
}
```

To show another embedded glTF model, add its bytes to `config.scene.models` and add an instance whose `model_index` points to that entry:

```rust
config.scene.models.push(include_bytes!("models/another.gltf"));
config.scene.instances.push(config::InstanceConfig {
    name: "Second model".to_owned(),
    model_index: 1,
    position: [0.5, 0.0, 0.0],
    rotation: [0.0; 3],
    scale: [0.5; 3],
    animation_axes: [false; 3],
});
```

Multiple instances may share a model and independently set their name, transform, and animated axes.

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

The tests cover loading and validating the embedded Pyraminx mesh and the renderer's triangle-picking intersection logic.

## Project layout

```text
src/
  app.rs                         Window and event-loop handling
  main.rs                        Application entry point
  config.rs                      Window, camera, scene, and instance configuration
  model.rs                       glTF mesh loading and instance transforms
  picking.rs                     Ray-based scene-object picking
  renderer.rs                    Vulkan rendering
  debug.rs                       ImGui debug and object-information windows
  models/pyraminx.gltf           Embedded, four-color Pyraminx mesh
  shaders/scene.vert.wgsl        Generic scene vertex shader source
  shaders/scene.frag.wgsl        Generic scene fragment shader source
build.rs                         Compiles WGSL shaders to SPIR-V
```

See [LICENSE](LICENSE) for licensing information.
