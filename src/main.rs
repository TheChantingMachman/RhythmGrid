mod gui;

use gui::app::App;
use gui::world::GameWorld;
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App {
        window: None,
        gpu: None,
        world: GameWorld::new(),
        pending_resize: None,
        resize_debounce: std::time::Instant::now(),
    };
    event_loop.run_app(&mut app).expect("event loop");
}
