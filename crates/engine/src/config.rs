use winit::keyboard::KeyCode;

use crate::scene::Scene;

pub struct AppConfig {
    pub window: WindowConfig,
    pub scene: Scene,
    pub shaders: ShaderConfig,
    pub debug_toggle_key: KeyCode,
    pub show_debug_menu_on_start: bool,
}

pub struct ShaderConfig {
    pub vertex: &'static [u8],
    pub fragment: &'static [u8],
}

pub struct WindowConfig {
    pub title: String,
    pub size: [u32; 2],
    pub resizable: bool,
}

pub struct SceneConfig {
    pub models: Vec<&'static [u8]>,
    pub instances: Vec<InstanceConfig>,
    pub background_color: [f32; 4],
    pub camera: CameraConfig,
}

#[derive(Clone, Copy)]
pub struct CameraConfig {
    pub projection: [f32; 2],
    pub distance: f32,
    pub depth_scale: f32,
}

pub struct InstanceConfig {
    pub name: String,
    pub model_index: usize,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}
