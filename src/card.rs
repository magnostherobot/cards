use cgmath::Vector3;
use strum::EnumIter;
use wgpu::{
    BufferUsages, Device, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use crate::{attributes, errors::*, util::create_buffer};

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum Suit {
    Clubs,
    Spades,
    Hearts,
    Diamonds,
}

impl Suit {
    pub fn doppelkopf_suit_strength(&self) -> u8 {
        match self {
            Suit::Clubs => 4,
            Suit::Spades => 3,
            Suit::Hearts => 2,
            Suit::Diamonds => 1,
        }
    }

    pub fn texture_index(&self) -> u8 {
        match self {
            Suit::Clubs => 3,
            Suit::Spades => 2,
            Suit::Hearts => 0,
            Suit::Diamonds => 1,
        }
    }
}

type Rank = u8;

pub struct Card {
    pub position: Vector3<i32>,
    pub facedown: bool,
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn to_instance(&self) -> Result<Instance> {
        Ok(Instance {
            model: cgmath::Matrix4::from_translation(
                self.position
                    .cast()
                    .chain_err(|| "couldn't cast card position vector")?,
            )
            .into(),
            rank: self.rank as u32,
            suit: self.suit.texture_index() as u32,
            facedown: self.facedown as u32,
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    model: [[f32; 4]; 4],
    rank: u32,
    suit: u32,
    facedown: u32,
}

impl Instance {
    pub const BUFFER_LAYOUT: VertexBufferLayout<'_> = {
        use std::mem::size_of;

        VertexBufferLayout {
            array_stride: size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &attributes!(
                start_location 5;
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Float32x4,
                VertexFormat::Uint32,
                VertexFormat::Uint32,
                VertexFormat::Uint32,
            ),
        }
    };
}

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
                tex_coords: [$x, (1.0 - $y)],
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
