// Bridge between winit key codes and pipeline's input::KeyCode.

use rhythm_grid::input::KeyCode as RgKeyCode;
use winit::keyboard::KeyCode as WinitKeyCode;

pub fn winit_to_rg(key: WinitKeyCode) -> RgKeyCode {
    match key {
        WinitKeyCode::ArrowLeft => RgKeyCode::Left,
        WinitKeyCode::ArrowRight => RgKeyCode::Right,
        WinitKeyCode::ArrowDown => RgKeyCode::Down,
        WinitKeyCode::ArrowUp => RgKeyCode::Up,
        WinitKeyCode::KeyZ => RgKeyCode::Z,
        WinitKeyCode::Space => RgKeyCode::Space,
        WinitKeyCode::KeyX => RgKeyCode::X,
        WinitKeyCode::KeyC => RgKeyCode::C,
        WinitKeyCode::KeyP => RgKeyCode::P,
        WinitKeyCode::Escape => RgKeyCode::Escape,
        WinitKeyCode::Enter => RgKeyCode::Enter,
        _ => RgKeyCode::Other,
    }
}
