mod texture;
use log::{debug, error};
use state::State;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod camera;
mod card;
mod state;
mod util;

use wgpu::SurfaceError;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod errors;
use errors::*;

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
    debug!("{event:?}");

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

async fn run_inner() -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .chain_err(|| "couldn't create new window")?;

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
