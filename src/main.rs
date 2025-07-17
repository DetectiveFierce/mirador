pub mod app;
pub mod game;
pub mod math;
pub mod maze;
pub mod renderer;
pub mod test_mode;
pub mod ui;
use winit::event_loop::{ControlFlow, EventLoop};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
}

async fn run() {
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            eprintln!("Error creating event loop: {}", err);
            return;
        }
    };

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app::App::new();

    event_loop.run_app(&mut app).expect("Failed to run app");
}
