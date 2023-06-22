use wgpu::{
    BufferUsages, Device, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::{attributes, util::create_buffer};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    pub const BUFFER_LAYOUT: VertexBufferLayout<'static> = {
        use std::mem::size_of;

        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &attributes![VertexFormat::Float32x3, VertexFormat::Float32x2],
        }
    };
}

// Cards are 34x48
pub const WIDTH: u32 = 34;
pub const HEIGHT: u32 = 48;

pub const VERTICES: &[Vertex] = {
    macro_rules! vert {
        ($x:expr, $y:expr $(,)?) => {{
            Vertex {
                position: [($x - 0.5) * WIDTH as f32, ($y - 0.5) * HEIGHT as f32, 0.0],
                tex_coords: [$x / 13.0, (1.0 - $y) / 4.0],
            }
        }};
    }

    &[
        vert!(1.0, 1.0),
        vert!(1.0, 0.0),
        vert!(0.0, 1.0),
        vert!(0.0, 0.0),
    ]
};

#[rustfmt::skip]
pub const INDICES: &[u16] = &[
    2, 3, 0,
    0, 3, 1,
];

pub fn create_vertex_buffer(device: &Device) -> wgpu::Buffer {
    create_buffer(device, "Card Vertex Buffer", VERTICES, BufferUsages::VERTEX)
}

pub fn create_index_buffer(device: &Device) -> wgpu::Buffer {
    create_buffer(device, "Card Index Buffer", INDICES, BufferUsages::INDEX)
}
