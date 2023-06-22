use bytemuck::cast_slice;
use cgmath::EuclideanSpace;
use log::info;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    Adapter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    BufferAddress, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, Device, DeviceDescriptor, Face, Features, FragmentState, FrontFace,
    IndexFormat, InstanceDescriptor, Limits, LoadOp, MultisampleState, Operations, PipelineLayout,
    PipelineLayoutDescriptor, PolygonMode, PowerPreference, PrimitiveState, PrimitiveTopology,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptionsBase, SamplerBindingType, ShaderModule,
    ShaderStages, Surface, SurfaceCapabilities, SurfaceConfiguration, SurfaceError, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
    VertexBufferLayout, VertexState, VertexStepMode,
};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraController, CameraUniform},
    card,
    errors::*,
    include_texture,
    texture::{self, Texture},
};

struct Instance {
    position: cgmath::Vector3<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: cgmath::Matrix4::from_translation(self.position).into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    const fn desc() -> VertexBufferLayout<'static> {
        use std::mem::size_of;

        VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

fn create_instance() -> wgpu::Instance {
    wgpu::Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        dx12_shader_compiler: Default::default(),
    })
}

async fn create_adapter(instance: &wgpu::Instance, surface: &Surface) -> Result<Adapter> {
    instance
        .request_adapter(&RequestAdapterOptionsBase {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .chain_err(|| "couldn't create adapter")
}

async fn create_logical_device_and_queue(adapter: &Adapter) -> Result<(Device, Queue)> {
    adapter
        .request_device(
            &DeviceDescriptor {
                features: Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    Limits::downlevel_webgl2_defaults()
                } else {
                    Limits::default()
                },
                label: None,
            },
            None,
        )
        .await
        .chain_err(|| "couldn't create logical device and queue")
}

fn get_surface_format(surface_caps: &SurfaceCapabilities) -> TextureFormat {
    surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.describe().srgb)
        .unwrap_or(surface_caps.formats[0])
}

fn create_pipeline_layout(
    device: &Device,
    texture_bind_group_layout: &BindGroupLayout,
    camera_bind_group_layout: &BindGroupLayout,
) -> PipelineLayout {
    device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[texture_bind_group_layout, camera_bind_group_layout],
        push_constant_ranges: &[],
    })
}

fn create_vertex_state(shader: &ShaderModule) -> VertexState {
    const VERTEX_BUFFERS: [VertexBufferLayout; 2] =
        [card::Vertex::BUFFER_LAYOUT, InstanceRaw::desc()];

    VertexState {
        module: shader,
        entry_point: "vs_main",
        buffers: &VERTEX_BUFFERS,
    }
}

fn create_fragment_state<'a>(
    shader: &'a ShaderModule,
    color_target_states: &'a [Option<ColorTargetState>],
) -> FragmentState<'a> {
    FragmentState {
        module: shader,
        entry_point: "fs_main",
        targets: color_target_states,
    }
}

fn create_primitive_state() -> PrimitiveState {
    PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: FrontFace::Ccw,
        cull_mode: Some(Face::Back),
        polygon_mode: PolygonMode::Fill,
        unclipped_depth: false,
        conservative: false,
    }
}

fn create_render_pipeline(
    device: &Device,
    config: &SurfaceConfiguration,
    texture_bind_group_layout: &BindGroupLayout,
    camera_bind_group_layout: &BindGroupLayout,
) -> RenderPipeline {
    let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
    let render_pipeline_layout =
        create_pipeline_layout(device, texture_bind_group_layout, camera_bind_group_layout);

    let color_target_states = &[Some(ColorTargetState {
        format: config.format,
        blend: Some(BlendState::ALPHA_BLENDING),
        write_mask: ColorWrites::ALL,
    })];

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: create_vertex_state(&shader),
        fragment: Some(create_fragment_state(&shader, color_target_states)),
        primitive: create_primitive_state(),
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}

fn create_texture_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("texture_bind_group_layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
            },
            BindGroupLayoutEntry {
                binding: 1,
                count: None,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
            },
        ],
    })
}

fn create_texture_bind_group(
    device: &Device,
    texture: &Texture,
    layout: &BindGroupLayout,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("diffuse_bind_group"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture.view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&texture.sampler),
            },
        ],
    })
}

fn create_camera(size: PhysicalSize<u32>) -> Camera {
    Camera {
        eye: cgmath::Point2::origin(),
        viewport_size: size,
        zoom: 2.0,
        znear: 0.1,
        zfar: 100.0,
    }
}

fn create_camera_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("camera_bind_group_layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform.to_owned(),
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn create_camera_bind_group(
    device: &Device,
    buffer: &wgpu::Buffer,
    layout: &BindGroupLayout,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("camera_bind_group"),
        layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

fn create_camera_buffer(device: &Device, uniform: CameraUniform) -> wgpu::Buffer {
    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: cast_slice(&[uniform]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    })
}

pub struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    window: Window,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_bind_group: BindGroup,
    _diffuse_texture: texture::Texture,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: BindGroup,
    camera_controller: CameraController,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
}

impl State {
    pub async fn new(window: Window) -> Result<Self> {
        let size = window.inner_size();

        let instance = create_instance();
        let surface =
            unsafe { instance.create_surface(&window) }.chain_err(|| "couldn't create surface")?;
        let adapter = create_adapter(&instance, &surface).await?;
        let (device, queue) = create_logical_device_and_queue(&adapter).await?;
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = get_surface_format(&surface_caps);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let diffuse_texture = include_texture!(&device, &queue, "cards.png")?;
        let texture_bind_group_layout = create_texture_bind_group_layout(&device);
        let diffuse_bind_group =
            create_texture_bind_group(&device, &diffuse_texture, &texture_bind_group_layout);

        let camera = create_camera(size);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        let camera_buffer = create_camera_buffer(&device, camera_uniform);
        let camera_bind_group_layout = create_camera_bind_group_layout(&device);
        let camera_bind_group =
            create_camera_bind_group(&device, &camera_buffer, &camera_bind_group_layout);

        let camera_controller = CameraController::new(2.0);

        let render_pipeline = create_render_pipeline(
            &device,
            &config,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
        );

        let vertex_buffer = card::create_vertex_buffer(&device);
        let index_buffer = card::create_index_buffer(&device);

        let num_indices = card::INDICES.len() as u32;

        let instances = (0..4)
            .flat_map(|suit| {
                (0..13).map(move |rank| {
                    let position = cgmath::Vector3::new(
                        1.2 * (card::WIDTH as f32) * (rank as f32 - 6.0),
                        1.2 * (card::HEIGHT as f32) * (suit as f32 - 1.5),
                        0.0,
                    );

                    Instance { position }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: cast_slice(&instance_data),
            usage: BufferUsages::VERTEX,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            diffuse_bind_group,
            _diffuse_texture: diffuse_texture,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            instances,
            instance_buffer,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.viewport_size = new_size;
        }

        info!(
            "set physical size to {}x{}",
            new_size.width, new_size.height
        );
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cast_slice(&[self.camera_uniform]));
    }

    pub fn render(&mut self) -> core::result::Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);

            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
