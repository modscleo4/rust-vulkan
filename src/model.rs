use std::error::Error;

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

pub(crate) fn load_gltf(source: &[u8]) -> Result<Model, Box<dyn Error>> {
    let (document, buffers, _) = gltf::import_slice(source)?;
    let name = document
        .nodes()
        .find(|node| node.mesh().is_some())
        .and_then(|node| node.name().map(str::to_owned))
        .ok_or("glTF model contains no named mesh node")?;
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

    #[test]
    fn pyraminx_gltf_loads_four_colored_faces() {
        let model = load_gltf(include_bytes!("models/pyraminx.gltf")).unwrap();

        assert_eq!(model.name, "Pyraminx");
        assert_eq!(model.vertices.len(), 12);
        assert_eq!(model.vertices[0].color, [0.95, 0.12, 0.12]);
        assert_eq!(model.vertices[3].color, [0.12, 0.32, 0.95]);
        assert_eq!(model.vertices[6].color, [0.98, 0.78, 0.08]);
        assert_eq!(model.vertices[9].color, [0.08, 0.72, 0.24]);
    }
}
