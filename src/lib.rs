mod texture;
use bytemuck::cast_slice;
use log::error;
use texture::Texture;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod camera;
use camera::{Camera, CameraController, CameraUniform};

use cgmath::prelude::*;

use std::{error::Error, fmt::Display};

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
    window::WindowBuilder,
};

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
    RenderPipelineDescriptor, RequestAdapterOptionsBase, RequestDeviceError, SamplerBindingType,
    ShaderModule, ShaderStages, Surface, SurfaceCapabilities, SurfaceConfiguration, SurfaceError,
    TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const fn desc() -> VertexBufferLayout<'static> {
        use std::mem::size_of;

        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
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

#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [ 0.1,  0.1, 0.0], tex_coords: [1.0, 0.0], }, // top right
    Vertex { position: [ 0.1, -0.1, 0.0], tex_coords: [1.0, 1.0], }, // bottom right
    Vertex { position: [-0.1,  0.1, 0.0], tex_coords: [0.0, 0.0], }, // top left
    Vertex { position: [-0.1, -0.1, 0.0], tex_coords: [0.0, 1.0], }, // bottom left
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    2, 3, 0,
    0, 3, 1,
];

fn create_instance() -> wgpu::Instance {
    wgpu::Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        dx12_shader_compiler: Default::default(),
    })
}

async fn create_adapter(
    instance: &wgpu::Instance,
    surface: &Surface,
) -> Result<Adapter, CreateAdapterError> {
    instance
        .request_adapter(&RequestAdapterOptionsBase {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .ok_or(CreateAdapterError {})
}

async fn create_logical_device_and_queue(
    adapter: &Adapter,
) -> Result<(Device, Queue), RequestDeviceError> {
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
}

fn create_buffer<A: bytemuck::Pod>(
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

#[derive(Debug)]
struct CreateAdapterError {}

impl Display for CreateAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error creating adapter")
    }
}

impl Error for CreateAdapterError {}

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
    const VERTEX_BUFFERS: [VertexBufferLayout; 2] = [Vertex::desc(), InstanceRaw::desc()];

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
        blend: Some(BlendState::REPLACE),
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

impl State {
    async fn new(window: Window) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();

        let instance = create_instance();
        let surface = unsafe { instance.create_surface(&window) }?;
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

        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_texture =
            Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png")?;

        let texture_bind_group_layout =
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
            });

        let diffuse_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&diffuse_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
        });

        let camera = Camera {
            eye: cgmath::Point2::origin(),
            aspect: size.width as f32 / size.height as f32,
            zoom: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
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
            });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let camera_controller = CameraController::new(0.2);

        let render_pipeline = create_render_pipeline(
            &device,
            &config,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
        );

        let vertex_buffer = create_buffer(&device, "Vertex Buffer", VERTICES, BufferUsages::VERTEX);
        let index_buffer = create_buffer(&device, "Index Buffer", INDICES, BufferUsages::INDEX);

        let num_indices = INDICES.len() as u32;

        const NUM_INSTANCES_PER_ROW: u32 = 10;
        const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
            0.0,
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
        );

        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = cgmath::Vector3 {
                        x: x as f32,
                        y: 0.0,
                        z: z as f32,
                    } - INSTANCE_DISPLACEMENT;

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance { position, rotation }
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
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cast_slice(&[self.camera_uniform]));
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
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

fn init_logging() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn)
                .expect("Couldn't initialise logger");
        } else {
            env_logger::init();
        }
    }
}

fn handle_window_event(state: &mut State, event: &WindowEvent) -> Option<ControlFlow> {
    match event {
        WindowEvent::CloseRequested
        | WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    ..
                },
            ..
        } => Some(ControlFlow::Exit),

        WindowEvent::Resized(physical_size) => {
            state.resize(*physical_size);
            None
        }

        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
            state.resize(**new_inner_size);
            None
        }

        _ => None,
    }
}

fn handle_redraw_event(state: &mut State) -> Option<ControlFlow> {
    state.update();
    match state.render() {
        Ok(_) => None,
        Err(SurfaceError::Lost) => {
            state.resize(state.size);
            None
        }
        Err(SurfaceError::OutOfMemory) => Some(ControlFlow::Exit),
        Err(e) => {
            eprintln!("{:?}", e);
            None
        }
    }
}

fn handle_event(state: &mut State, event: &Event<()>) -> Option<ControlFlow> {
    match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if *window_id == state.window().id() && !state.input(event) => {
            handle_window_event(state, event)
        }

        Event::RedrawRequested(window_id) if *window_id == state.window().id() => {
            handle_redraw_event(state)
        }

        Event::MainEventsCleared => {
            state.window().request_redraw();
            None
        }

        _ => None,
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    init_logging();

    match run_inner().await {
        Ok(_) => (),
        Err(e) => error!("{e:?}"),
    }
}

async fn run_inner() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        window.set_inner_size(winit::dpi::LogicalSize::new(800, 600));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = State::new(window).await?;

    event_loop.run(move |event, _, control_flow| {
        if let Some(new_flow) = handle_event(&mut state, &event) {
            *control_flow = new_flow;
        }
    });
}
