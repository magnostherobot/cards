use std::{error::Error, fmt::Display};

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
    window::WindowBuilder,
};

use wgpu::{
    Surface, Device, Queue, Adapter, Instance, RequestDeviceError,
    SurfaceCapabilities, TextureFormat, RenderPipeline, SurfaceConfiguration,
    PipelineLayout, ShaderModule, VertexState, FragmentState, ColorTargetState,
    PrimitiveState, util::{DeviceExt, BufferInitDescriptor}, VertexBufferLayout, VertexAttribute, BufferUsages,
};

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    render_pipeline: RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
];

const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

fn create_instance() -> Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
    })
}

async fn create_adapter(instance: &Instance, surface: &Surface) -> Result<Adapter, CreateAdapterError> {
    instance.request_adapter(
        &wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        },
    )
        .await
        .ok_or(CreateAdapterError {})
}

async fn create_logical_device_and_queue(adapter: &Adapter) -> Result<(Device, Queue), RequestDeviceError> {
    adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None,
        ).await
}

fn create_buffer<A: bytemuck::Pod>(
    device: &Device,
    name: &str,
    contents: &[A],
    usage: BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(
        &BufferInitDescriptor {
            label: Some(name),
            contents: bytemuck::cast_slice(contents),
            usage
        }
    )
}

#[derive(Debug)]
struct CreateAdapterError {
}

impl Display for CreateAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error creating adapter")
    }
}

impl Error for CreateAdapterError {
}

fn get_surface_format(surface_caps: &SurfaceCapabilities) -> TextureFormat {
    surface_caps.formats.iter()
        .copied()
        .find(|f| f.describe().srgb)
        .unwrap_or(surface_caps.formats[0])
}

fn create_pipeline_layout(device: &Device) -> PipelineLayout {
    device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        }
    )
}

fn create_vertex_state(shader: &ShaderModule) -> VertexState {
    const VERTEX_BUFFERS: [VertexBufferLayout; 1] = [
        Vertex::desc(),
    ];

    wgpu::VertexState {
        module: shader,
        entry_point: "vs_main",
        buffers: &VERTEX_BUFFERS,
    }
}

fn create_fragment_state<'a>(
    shader: &'a ShaderModule,
    color_target_states: &'a[Option<ColorTargetState>],
) -> FragmentState<'a> {
    FragmentState {
        module: shader,
        entry_point: "fs_main",
        targets: color_target_states,
    }
}

fn create_primitive_state() -> PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        polygon_mode: wgpu::PolygonMode::Fill,
        unclipped_depth: false,
        conservative: false,
    }
}

fn create_render_pipeline(device: &Device, config: &SurfaceConfiguration) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let render_pipeline_layout = create_pipeline_layout(device);

    let color_target_states = &[
        Some(wgpu::ColorTargetState {
            format: config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ];

    device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: create_vertex_state(&shader),
            fragment: Some(create_fragment_state(&shader, color_target_states)),
            primitive: create_primitive_state(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        }
    )
}

impl State {
    async fn create(window: Window) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();

        let instance = create_instance();
        let surface = unsafe { instance.create_surface(&window) }?;
        let adapter = create_adapter(&instance, &surface).await?;
        let (device, queue) = create_logical_device_and_queue(&adapter).await?;
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = get_surface_format(&surface_caps);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu:: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let render_pipeline = create_render_pipeline(&device, &config);

        let vertex_buffer = create_buffer(&device, "Vertex Buffer", VERTICES, BufferUsages::VERTEX);
        let index_buffer = create_buffer(&device, "Index Buffer", INDICES, BufferUsages::INDEX);

        let num_indices = INDICES.len() as u32;

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
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config)
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        // empty
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
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
        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
            input: KeyboardInput {
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

        _ => None
    }
}

fn handle_redraw_event(state: &mut State) -> Option<ControlFlow> {
    state.update();
    match state.render() {
        Ok(_) => None,
        Err(wgpu::SurfaceError::Lost) => { state.resize(state.size); None },
        Err(wgpu::SurfaceError::OutOfMemory) => Some(ControlFlow::Exit),
        Err(e) => { eprintln!("{:?}", e); None },
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

        _ => None
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() -> Result<(), Box<dyn Error>> {
    init_logging();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;

    let mut state = State::create(window).await?;

    #[cfg(target_arch="wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));
        
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

    event_loop.run(move |event, _, control_flow| {
        if let Some(new_flow) = handle_event(&mut state, &event) {
            *control_flow = new_flow;
        }
    });
}
