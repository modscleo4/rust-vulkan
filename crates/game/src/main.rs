use vulkan_engine::{
    AppConfig, CameraConfig, InstanceConfig, Scene, SceneConfig, ShaderConfig, WindowConfig,
};
use winit::keyboard::KeyCode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    vulkan_engine::run(game_config())
}

fn game_config() -> AppConfig {
    let layout = [
        ("X", [-0.33, 0.58, 0.0], [true, false, false]),
        ("Y", [0.33, 0.58, 0.0], [false, true, false]),
        ("Z", [-0.66, 0.0, 0.0], [false, false, true]),
        ("XY", [0.0, 0.0, 0.0], [true, true, false]),
        ("XZ", [0.66, 0.0, 0.0], [true, false, true]),
        ("YZ", [-0.33, -0.58, 0.0], [false, true, true]),
        ("XYZ", [0.33, -0.58, 0.0], [true, true, true]),
    ];

    let animation_axes = layout.map(|(_, _, animation_axes)| animation_axes);
    let scene = Scene::new(
        SceneConfig {
            models: vec![include_bytes!("../assets/pyraminx.gltf")],
            instances: layout
                .into_iter()
                .map(|(name, position, _)| InstanceConfig {
                    name: name.to_owned(),
                    model_index: 0,
                    position,
                    rotation: [0.0; 3],
                    scale: [0.42; 3],
                })
                .collect(),
            background_color: [0.03, 0.03, 0.06, 1.0],
            camera: CameraConfig {
                projection: [1.05, 1.4],
                distance: 3.0,
                depth_scale: 1.333333,
            },
        },
        |instances| {
            for instance in instances {
                instance.rotation = [0.0; 3];
            }
        },
        {
            let mut elapsed_seconds = 0.0;
            move |instances, delta_time| {
                elapsed_seconds += delta_time;
                let angle = (elapsed_seconds * 0.8).rem_euclid(std::f32::consts::TAU);
                for (instance, animation_axes) in instances.iter_mut().zip(animation_axes) {
                    for (rotation, animated) in instance.rotation.iter_mut().zip(animation_axes) {
                        if animated {
                            *rotation = angle;
                        }
                    }
                }
            }
        },
    );

    AppConfig {
        window: WindowConfig {
            title: "Rotating Pyraminx".to_owned(),
            size: [800, 600],
            resizable: false,
        },
        scene,
        shaders: ShaderConfig {
            vertex: include_bytes!(concat!(env!("OUT_DIR"), "/scene.vert.spv")),
            fragment: include_bytes!(concat!(env!("OUT_DIR"), "/scene.frag.spv")),
        },
        debug_toggle_key: KeyCode::F3,
        show_debug_menu_on_start: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_configuration_preserves_the_default_scene() {
        let config = game_config();

        assert_eq!(config.window.size, [800, 600]);
        assert_eq!(config.scene.config().models.len(), 1);
        assert_eq!(config.scene.config().instances.len(), 7);
        assert!(
            config
                .scene
                .config()
                .instances
                .iter()
                .all(|instance| instance.model_index == 0)
        );
        assert_eq!(config.scene.config().camera.distance, 3.0);
    }

    #[test]
    fn scene_lifecycle_drives_instance_animation() {
        let mut config = game_config();
        config.scene.on_start();
        config.scene.on_update(0.25);

        assert_eq!(config.scene.config().instances[0].rotation[0], 0.2);
        assert_eq!(config.scene.config().instances[0].rotation[1], 0.0);
        assert_eq!(config.scene.config().instances[1].rotation[1], 0.2);
    }
}
