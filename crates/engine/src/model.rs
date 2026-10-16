use std::{error::Error, sync::Arc};

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
}

pub(crate) struct Model {
    pub(crate) name: String,
    pub(crate) vertices: Vec<Vertex>,
}

pub(crate) struct Instance {
    pub(crate) model: Arc<Model>,
    pub(crate) name: String,
    pub(crate) position: [f32; 3],
    pub(crate) rotation: [f32; 3],
    pub(crate) scale: [f32; 3],
    pub(crate) model_index: usize,
}

impl Instance {
    pub(crate) fn new(
        model: Arc<Model>,
        name: String,
        position: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
        model_index: usize,
    ) -> Self {
        Self {
            model,
            name,
            position,
            rotation,
            scale,
            model_index,
        }
    }
}

pub(crate) fn load_gltf(source: &[u8]) -> Result<Model, Box<dyn Error>> {
    let (document, buffers, _) = gltf::import_slice(source)?;
    let name = document
        .nodes()
        .find(|node| node.mesh().is_some())
        .and_then(|node| node.name().map(str::to_owned))
        .or_else(|| {
            document
                .meshes()
                .find_map(|mesh| mesh.name().map(str::to_owned))
        })
        .unwrap_or_else(|| "Unnamed model".to_owned());
    let mut vertices = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                return Err(format!(
                    "glTF mesh '{}' uses unsupported primitive mode {:?}",
                    mesh.name().unwrap_or("<unnamed>"),
                    primitive.mode()
                )
                .into());
            }

            let reader = primitive
                .reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
            let positions = reader
                .read_positions()
                .ok_or("glTF primitive is missing POSITION data")?
                .map(|position| [position[0], position[1], position[2]])
                .collect::<Vec<_>>();
            let indices = reader
                .read_indices()
                .map(|indices| indices.into_u32().collect::<Vec<_>>())
                .unwrap_or_else(|| (0..positions.len() as u32).collect());
            if indices.len() % 3 != 0 {
                return Err("glTF triangle primitive has an incomplete triangle".into());
            }

            let color = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_factor();
            for triangle in indices.chunks_exact(3) {
                for &index in triangle {
                    let position = positions
                        .get(index as usize)
                        .ok_or("glTF primitive index is out of range")?;
                    vertices.push(Vertex {
                        position: *position,
                        color: [color[0], color[1], color[2]],
                    });
                }
            }
        }
    }

    if vertices.is_empty() {
        return Err("glTF model contains no triangles".into());
    }
    Ok(Model { name, vertices })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn instances_share_models_and_store_transforms() {
        let model = Arc::new(Model {
            name: "Test".to_owned(),
            vertices: Vec::new(),
        });
        let instances = [
            Instance::new(
                Arc::clone(&model),
                "Test".to_owned(),
                [-0.33, 0.58, 0.0],
                [0.0; 3],
                [0.42; 3],
                0,
            ),
            Instance::new(
                Arc::clone(&model),
                "Second".to_owned(),
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                0,
            ),
        ];

        assert!(
            instances
                .iter()
                .all(|instance| Arc::ptr_eq(&instance.model, &model))
        );
        assert_eq!(instances[0].position, [-0.33, 0.58, 0.0]);
        assert_eq!(instances[0].rotation, [0.0; 3]);
        assert_eq!(instances[0].scale, [0.42; 3]);
        assert_eq!(instances[0].model_index, 0);
        assert_eq!(instances[1].model_index, 0);
    }

    #[test]
    fn pyraminx_gltf_loads_four_colored_faces() {
        let model = load_gltf(include_bytes!("../tests/fixtures/pyraminx.gltf")).unwrap();

        assert_eq!(model.name, "Pyraminx");
        assert_eq!(model.vertices.len(), 12);
        assert_eq!(model.vertices[0].color, [0.95, 0.12, 0.12]);
        assert_eq!(model.vertices[3].color, [0.12, 0.32, 0.95]);
        assert_eq!(model.vertices[6].color, [0.98, 0.78, 0.08]);
        assert_eq!(model.vertices[9].color, [0.08, 0.72, 0.24]);
    }
}
