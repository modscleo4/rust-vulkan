use winit::keyboard::KeyCode;

pub(crate) struct AppConfig {
    pub(crate) window: WindowConfig,
    pub(crate) scene: SceneConfig,
    pub(crate) debug_toggle_key: KeyCode,
    pub(crate) show_debug_menu_on_start: bool,
}

pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) size: [u32; 2],
    pub(crate) resizable: bool,
}

pub(crate) struct SceneConfig {
    pub(crate) models: Vec<&'static [u8]>,
    pub(crate) instances: Vec<InstanceConfig>,
    pub(crate) animation_speed: f32,
    pub(crate) background_color: [f32; 4],
    pub(crate) camera: CameraConfig,
}

#[derive(Clone, Copy)]
pub(crate) struct CameraConfig {
    pub(crate) projection: [f32; 2],
    pub(crate) distance: f32,
    pub(crate) depth_scale: f32,
}

pub(crate) struct InstanceConfig {
    pub(crate) name: String,
    pub(crate) model_index: usize,
    pub(crate) position: [f32; 3],
    pub(crate) rotation: [f32; 3],
    pub(crate) scale: [f32; 3],
    pub(crate) animation_axes: [bool; 3],
}

impl Default for AppConfig {
    fn default() -> Self {
        let layout = [
            ("X", [-0.33, 0.58, 0.0], [true, false, false]),
            ("Y", [0.33, 0.58, 0.0], [false, true, false]),
            ("Z", [-0.66, 0.0, 0.0], [false, false, true]),
            ("XY", [0.0, 0.0, 0.0], [true, true, false]),
            ("XZ", [0.66, 0.0, 0.0], [true, false, true]),
            ("YZ", [-0.33, -0.58, 0.0], [false, true, true]),
            ("XYZ", [0.33, -0.58, 0.0], [true, true, true]),
        ];

        Self {
            window: WindowConfig {
                title: "Rotating Pyraminx".to_owned(),
                size: [800, 600],
                resizable: false,
            },
            scene: SceneConfig {
                models: vec![include_bytes!("models/pyraminx.gltf")],
                instances: layout
                    .into_iter()
                    .map(|(name, position, animation_axes)| InstanceConfig {
                        name: name.to_owned(),
                        model_index: 0,
                        position,
                        rotation: [0.0; 3],
                        scale: [0.42; 3],
                        animation_axes,
                    })
                    .collect(),
                animation_speed: 0.8,
                background_color: [0.03, 0.03, 0.06, 1.0],
                camera: CameraConfig {
                    projection: [1.05, 1.4],
                    distance: 3.0,
                    depth_scale: 1.333333,
                },
            },
            debug_toggle_key: KeyCode::F3,
            show_debug_menu_on_start: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_builds_the_original_scene() {
        let config = AppConfig::default();

        assert_eq!(config.window.size, [800, 600]);
        assert_eq!(config.scene.models.len(), 1);
        assert_eq!(config.scene.instances.len(), 7);
        assert_eq!(config.scene.instances[0].model_index, 0);
        assert_eq!(config.scene.animation_speed, 0.8);
        assert_eq!(config.scene.camera.distance, 3.0);
    }
}
