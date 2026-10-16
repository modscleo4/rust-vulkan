use crate::config::{InstanceConfig, SceneConfig};

pub struct Scene {
    config: SceneConfig,
    on_start: Option<Box<dyn FnMut(&mut [InstanceConfig])>>,
    on_update: Option<Box<dyn FnMut(&mut [InstanceConfig], f32)>>,
}

impl Scene {
    pub fn new(
        config: SceneConfig,
        on_start: impl FnMut(&mut [InstanceConfig]) + 'static,
        on_update: impl FnMut(&mut [InstanceConfig], f32) + 'static,
    ) -> Self {
        Self {
            config,
            on_start: Some(Box::new(on_start)),
            on_update: Some(Box::new(on_update)),
        }
    }

    pub fn config(&self) -> &SceneConfig {
        &self.config
    }

    pub fn on_start(&mut self) {
        if let Some(callback) = &mut self.on_start {
            callback(&mut self.config.instances);
        }
    }

    pub fn on_update(&mut self, delta_time: f32) {
        if let Some(callback) = &mut self.on_update {
            callback(&mut self.config.instances, delta_time);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CameraConfig;

    fn test_scene() -> Scene {
        Scene::new(
            SceneConfig {
                models: vec![],
                instances: vec![InstanceConfig {
                    name: "test".to_owned(),
                    model_index: 0,
                    position: [0.0; 3],
                    rotation: [0.0; 3],
                    scale: [1.0; 3],
                }],
                background_color: [0.0; 4],
                camera: CameraConfig {
                    projection: [1.0; 2],
                    distance: 1.0,
                    depth_scale: 1.0,
                },
            },
            |instances| instances[0].position[0] = 1.0,
            |instances, delta_time| {
                instances[0].position[0] += delta_time;
            },
        )
    }

    #[test]
    fn lifecycle_callbacks_receive_startup_and_update_events() {
        let mut scene = test_scene();
        scene.on_start();
        assert_eq!(scene.config().instances[0].position[0], 1.0);

        scene.on_update(0.25);
        assert_eq!(scene.config().instances[0].position[0], 1.25);
    }
}
