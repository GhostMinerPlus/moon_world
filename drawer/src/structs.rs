use std::f32::consts::PI;

use nalgebra::{point, vector, Matrix4, Vector4};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Line {
    pub sp: [f32; 2],
    pub ep: [f32; 2],

    pub light: f32,
    pub color: [f32; 3],

    pub roughness: f32,
    pub seed: f32,
    pub _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInput {
    pub position: [f32; 2],
}

impl PointInput {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Point3Input {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub normal: [f32; 4],
}

impl Point3Input {
    const POS_ONLY_ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x4];
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];

    pub fn pos_only_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::POS_ONLY_ATTRIBS,
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct Point3InputArray {
    vertex_v: Vec<Point3Input>,
}

impl Point3InputArray {
    pub fn cube(color: Vector4<f32>) -> Point3InputArray {
        let color = [color.x, color.y, color.z, color.w];
        let normal = [0.0, 0.0, 1.0, 0.0];

        let mut vertex_v = vec![
            Point3Input {
                position: [0.0, 0.0, 0.0, 1.0],
                color,
                normal,
            },
            Point3Input {
                position: [1.0, 0.0, 0.0, 1.0],
                color,
                normal,
            },
            Point3Input {
                position: [0.0, 1.0, 0.0, 1.0],
                color,
                normal,
            },
            Point3Input {
                position: [0.0, 1.0, 0.0, 1.0],
                color,
                normal,
            },
            Point3Input {
                position: [1.0, 0.0, 0.0, 1.0],
                color,
                normal,
            },
            Point3Input {
                position: [1.0, 1.0, 0.0, 1.0],
                color,
                normal,
            },
        ];

        let mut cur_is_left = true;

        for i in 1..6 {
            let offet = (i - 1) * 6;

            let o_face = &vertex_v[offet..offet + 6];

            let p_o = point![
                o_face[0].position[0],
                o_face[0].position[1],
                o_face[0].position[2],
            ];
            let p_x = point![
                o_face[1].position[0],
                o_face[1].position[1],
                o_face[1].position[2],
            ];
            let p_y = point![
                o_face[2].position[0],
                o_face[2].position[1],
                o_face[2].position[2],
            ];

            let ox = p_x - p_o;
            let oy = p_y - p_o;

            let r_m = if cur_is_left {
                Matrix4::new_rotation_wrt_point(-oy * 0.5 * PI, p_x)
            } else {
                Matrix4::new_rotation_wrt_point(-ox * 0.5 * PI, p_o)
            };
            let rt_m = if cur_is_left {
                Matrix4::new_translation(&(-ox)) * r_m
            } else {
                Matrix4::new_translation(&oy) * r_m
            };

            let face = o_face
                .iter()
                .map(|vertex| {
                    let position = rt_m.transform_point(&point![
                        vertex.position[0],
                        vertex.position[1],
                        vertex.position[2],
                    ]);
                    let normal = rt_m
                        .transform_vector(&vector![
                            vertex.normal[0],
                            vertex.normal[1],
                            vertex.normal[2],
                        ])
                        .normalize();

                    Point3Input {
                        position: [position.x, position.y, position.z, 1.0],
                        color: vertex.color,
                        normal: [normal.x, normal.y, normal.z, 0.0],
                    }
                })
                .collect::<Vec<Point3Input>>();

            vertex_v.extend(face);
            cur_is_left = !cur_is_left;
        }

        Self { vertex_v }
    }

    pub fn vertex_v(&self) -> &[Point3Input] {
        &self.vertex_v
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineIn {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

impl LineIn {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
