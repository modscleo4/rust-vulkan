use std::error::Error;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
}

pub(crate) fn parse_obj(source: &str) -> Result<Vec<Vertex>, Box<dyn Error>> {
    let mut positions = Vec::new();
    let mut vertices = Vec::new();
    let mut color = [0.8, 0.8, 0.8];

    for (line_number, line) in source.lines().enumerate() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("v") => {
                let coordinates = fields
                    .take(3)
                    .map(str::parse::<f32>)
                    .collect::<Result<Vec<_>, _>>()?;
                if coordinates.len() != 3 {
                    return Err(format!(
                        "OBJ line {}: expected 3 vertex coordinates",
                        line_number + 1
                    )
                    .into());
                }
                positions.push([coordinates[0], coordinates[1], coordinates[2]]);
            }
            Some("g") | Some("usemtl") => {
                color = match fields.next().unwrap_or_default() {
                    "red" => [0.95, 0.12, 0.12],
                    "blue" => [0.12, 0.32, 0.95],
                    "yellow" => [0.98, 0.78, 0.08],
                    "green" => [0.08, 0.72, 0.24],
                    _ => [0.8, 0.8, 0.8],
                };
            }
            Some("f") => {
                let indices = fields
                    .map(|field| {
                        let raw_index = field
                            .split('/')
                            .next()
                            .unwrap_or_default()
                            .parse::<isize>()?;
                        let index = if raw_index > 0 {
                            raw_index - 1
                        } else {
                            positions.len() as isize + raw_index
                        };
                        usize::try_from(index).map_err(|_| {
                            format!("OBJ line {}: invalid vertex index", line_number + 1).into()
                        })
                    })
                    .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
                if indices.len() < 3 {
                    return Err(format!(
                        "OBJ line {}: face needs at least 3 vertices",
                        line_number + 1
                    )
                    .into());
                }
                for index in &indices {
                    if *index >= positions.len() {
                        return Err(format!(
                            "OBJ line {}: vertex index is out of range",
                            line_number + 1
                        )
                        .into());
                    }
                }
                for corner in 1..indices.len() - 1 {
                    for index in [indices[0], indices[corner], indices[corner + 1]] {
                        vertices.push(Vertex {
                            position: positions[index],
                            color,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err("OBJ model contains no faces".into());
    }
    Ok(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyraminx_obj_loads_four_colored_faces() {
        let vertices = parse_obj(include_str!("models/pyraminx.obj")).unwrap();

        assert_eq!(vertices.len(), 12);
        assert_eq!(vertices[0].color, [0.95, 0.12, 0.12]);
        assert_eq!(vertices[3].color, [0.12, 0.32, 0.95]);
        assert_eq!(vertices[6].color, [0.98, 0.78, 0.08]);
        assert_eq!(vertices[9].color, [0.08, 0.72, 0.24]);
    }

    #[test]
    fn obj_parser_triangulates_polygon_faces() {
        let vertices =
            parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\ng blue\nf 1 2 3 4\n").unwrap();

        assert_eq!(vertices.len(), 6);
        assert!(
            vertices
                .iter()
                .all(|vertex| vertex.color == [0.12, 0.32, 0.95])
        );
    }
}
