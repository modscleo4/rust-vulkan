# Rotating Pyraminx

A small Rust desktop application that renders seven continuously rotating, four-color Pyraminx models using Vulkan. Each rotates around a different combination of the X, Y, and Z axes (X, Y, Z, XY, XZ, YZ, and XYZ). The model is loaded from an embedded glTF asset, and a Dear ImGui debug overlay provides basic runtime and object information.

## Requirements

- Rust stable with Cargo (Rust 2024 edition support)
- A Vulkan 1.0-compatible graphics device and its installed Vulkan loader/driver

The Vulkan SDK is not needed to build the project; WGSL shaders are compiled to SPIR-V as part of the Cargo build.

## Build and run

From the project directory:

```powershell
cargo run
```

Cargo builds the application and launches it. The window is 800 × 600 and is not resizable. The model and shader sources are included in the repository, so no separate asset download is required.

## Controls

- **F3** — Show or hide the debug overlay.
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
  model.rs                       glTF mesh loading
  renderer.rs                    Vulkan rendering and debug UI
  models/pyraminx.gltf           Embedded, four-color Pyraminx mesh
  shaders/pyraminx.vert.wgsl     Vertex shader source
  shaders/pyraminx.frag.wgsl     Fragment shader source
build.rs                         Compiles WGSL shaders to SPIR-V
```

See [LICENSE](LICENSE) for licensing information.
