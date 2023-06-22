use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, Device, VertexFormat,
};

/// Counts the number of arguments passed to it.
#[macro_export]
macro_rules! count {
    () => {
        0usize
    };

    ($_head:tt $($tail:tt)*) => {
        1usize + count!($($tail)*)
    };
}

/// Creates an array of VertexAttributes, offset correctly based on the formats passed in as arguments.
#[macro_export]
macro_rules! attributes {
    ( $( $x:expr ),* ) => {{
        use $crate::util::vertex_format_size;
        use $crate::count;

        let mut shader_location: u32 = 0;
        let mut offset: u64 = 0;
        const ATTR_COUNT: usize = count!($($x)*);

        let mut data: [VertexAttribute; ATTR_COUNT] = [VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: VertexFormat::Float32,
        }; ATTR_COUNT];

        $(
            #[allow(unused_assignments)]
            {
                data[shader_location as usize] = VertexAttribute {
                    offset,
                    shader_location,
                    format: $x,
                };

                shader_location += 1;
                offset += vertex_format_size($x) as u64;
            }
        )*

        data
    }}
}

pub fn create_buffer<A: bytemuck::Pod>(
    device: &Device,
    name: &str,
    contents: &[A],
    usage: BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&BufferInitDescriptor {
        label: Some(name),
        contents: bytemuck::cast_slice(contents),
        usage,
    })
}

pub const fn vertex_format_size(format: VertexFormat) -> usize {
    use std::mem::size_of;

    match format {
        VertexFormat::Float16x2 => todo!(),
        VertexFormat::Float16x4 => todo!(),
        VertexFormat::Float32 => size_of::<f32>(),
        VertexFormat::Float32x2 => size_of::<[f32; 2]>(),
        VertexFormat::Float32x3 => size_of::<[f32; 3]>(),
        VertexFormat::Float32x4 => size_of::<[f32; 4]>(),
        VertexFormat::Float64 => size_of::<f64>(),
        VertexFormat::Float64x2 => size_of::<[f64; 2]>(),
        VertexFormat::Float64x3 => size_of::<[f64; 3]>(),
        VertexFormat::Float64x4 => size_of::<[f64; 4]>(),
        VertexFormat::Sint16x2 => size_of::<[i16; 2]>(),
        VertexFormat::Sint16x4 => size_of::<[i16; 4]>(),
        VertexFormat::Sint32 => size_of::<i32>(),
        VertexFormat::Sint32x2 => size_of::<[i32; 2]>(),
        VertexFormat::Sint32x3 => size_of::<[i32; 3]>(),
        VertexFormat::Sint32x4 => size_of::<[i32; 4]>(),
        VertexFormat::Sint8x2 => size_of::<[i8; 2]>(),
        VertexFormat::Sint8x4 => size_of::<[i8; 4]>(),
        VertexFormat::Snorm16x2 => size_of::<[i16; 2]>(),
        VertexFormat::Snorm16x4 => size_of::<[i16; 4]>(),
        VertexFormat::Snorm8x2 => size_of::<[i8; 2]>(),
        VertexFormat::Snorm8x4 => size_of::<[i8; 4]>(),
        VertexFormat::Uint16x2 => size_of::<[u16; 2]>(),
        VertexFormat::Uint16x4 => size_of::<[u16; 4]>(),
        VertexFormat::Uint32 => size_of::<u32>(),
        VertexFormat::Uint32x2 => size_of::<[u32; 2]>(),
        VertexFormat::Uint32x3 => size_of::<[u32; 3]>(),
        VertexFormat::Uint32x4 => size_of::<[u32; 4]>(),
        VertexFormat::Uint8x2 => size_of::<[u8; 2]>(),
        VertexFormat::Uint8x4 => size_of::<[u8; 4]>(),
        VertexFormat::Unorm16x2 => size_of::<[u16; 2]>(),
        VertexFormat::Unorm16x4 => size_of::<[u16; 4]>(),
        VertexFormat::Unorm8x2 => size_of::<[u8; 2]>(),
        VertexFormat::Unorm8x4 => size_of::<[u8; 4]>(),
    }
}
