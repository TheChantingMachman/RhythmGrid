use std::sync::Arc;
use std::time::Instant;

use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::input::{self, GameAction, KeyCode as RgKeyCode};
use rhythm_grid::pieces::*;
use rhythm_grid::render::*;

// --- Layout constants ---
const SIDEBAR_W: u32 = 160;
const WIN_W: u32 = BOARD_WIDTH_PX + SIDEBAR_W;
const WIN_H: u32 = BOARD_HEIGHT_PX;

// --- GPU Vertex ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// --- Shader ---
const SHADER_SRC: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

// --- Tiny 3x5 bitmap font for HUD ---
const FONT: &[(char, [u8; 5])] = &[
    ('0', [0b111, 0b101, 0b101, 0b101, 0b111]),
    ('1', [0b010, 0b110, 0b010, 0b010, 0b111]),
    ('2', [0b111, 0b001, 0b111, 0b100, 0b111]),
    ('3', [0b111, 0b001, 0b111, 0b001, 0b111]),
    ('4', [0b101, 0b101, 0b111, 0b001, 0b001]),
    ('5', [0b111, 0b100, 0b111, 0b001, 0b111]),
    ('6', [0b111, 0b100, 0b111, 0b101, 0b111]),
    ('7', [0b111, 0b001, 0b010, 0b010, 0b010]),
    ('8', [0b111, 0b101, 0b111, 0b101, 0b111]),
    ('9', [0b111, 0b101, 0b111, 0b001, 0b111]),
    ('A', [0b010, 0b101, 0b111, 0b101, 0b101]),
    ('B', [0b110, 0b101, 0b110, 0b101, 0b110]),
    ('C', [0b111, 0b100, 0b100, 0b100, 0b111]),
    ('D', [0b110, 0b101, 0b101, 0b101, 0b110]),
    ('E', [0b111, 0b100, 0b111, 0b100, 0b111]),
    ('F', [0b111, 0b100, 0b111, 0b100, 0b100]),
    ('G', [0b111, 0b100, 0b101, 0b101, 0b111]),
    ('H', [0b101, 0b101, 0b111, 0b101, 0b101]),
    ('I', [0b111, 0b010, 0b010, 0b010, 0b111]),
    ('K', [0b101, 0b110, 0b100, 0b110, 0b101]),
    ('L', [0b100, 0b100, 0b100, 0b100, 0b111]),
    ('M', [0b101, 0b111, 0b111, 0b101, 0b101]),
    ('N', [0b101, 0b111, 0b111, 0b101, 0b101]),
    ('O', [0b111, 0b101, 0b101, 0b101, 0b111]),
    ('P', [0b111, 0b101, 0b111, 0b100, 0b100]),
    ('R', [0b111, 0b101, 0b111, 0b110, 0b101]),
    ('S', [0b111, 0b100, 0b111, 0b001, 0b111]),
    ('T', [0b111, 0b010, 0b010, 0b010, 0b010]),
    ('U', [0b101, 0b101, 0b101, 0b101, 0b111]),
    ('V', [0b101, 0b101, 0b101, 0b101, 0b010]),
    ('W', [0b101, 0b101, 0b111, 0b111, 0b101]),
    ('X', [0b101, 0b101, 0b010, 0b101, 0b101]),
    ('Z', [0b111, 0b001, 0b010, 0b100, 0b111]),
    ('-', [0b000, 0b000, 0b111, 0b000, 0b000]),
    (':', [0b000, 0b010, 0b000, 0b010, 0b000]),
    (' ', [0b000, 0b000, 0b000, 0b000, 0b000]),
];

// --- Coordinate helpers ---
fn px_to_ndc(px_x: f32, px_y: f32, win_w: f32, win_h: f32) -> (f32, f32) {
    let nx = (px_x / win_w) * 2.0 - 1.0;
    let ny = 1.0 - (px_y / win_h) * 2.0;
    (nx, ny)
}

fn rgba_to_f32(c: [u8; 4]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0]
}

fn push_quad(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
             x: f32, y: f32, w: f32, h: f32, color: [f32; 4], z: f32) {
    let ww = WIN_W as f32;
    let wh = WIN_H as f32;
    let (x0, y0) = px_to_ndc(x, y, ww, wh);
    let (x1, y1) = px_to_ndc(x + w, y + h, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x0, y0, z], color });
    verts.push(Vertex { position: [x1, y0, z], color });
    verts.push(Vertex { position: [x1, y1, z], color });
    verts.push(Vertex { position: [x0, y1, z], color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

fn push_text(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
             x: f32, y: f32, text: &str, color: [f32; 4], scale: f32) {
    for (i, ch) in text.chars().enumerate() {
        let upper = ch.to_ascii_uppercase();
        let glyph = FONT.iter().find(|(c, _)| *c == upper).map(|(_, g)| g);
        if let Some(glyph) = glyph {
            let cx = x + i as f32 * 4.0 * scale;
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..3 {
                    if bits & (1 << (2 - col)) != 0 {
                        push_quad(verts, indices,
                            cx + col as f32 * scale,
                            y + row as f32 * scale,
                            scale, scale, color, 0.0);
                    }
                }
            }
        }
    }
}

// --- Vanish-zone-aware helpers (same as before, pipeline's is_valid_position rejects r<0) ---
fn is_valid_position_vz(grid: &Grid, cells: &[(i32, i32)], row: i32, col: i32) -> bool {
    for &(dr, dc) in cells {
        let r = row + dr;
        let c = col + dc;
        if c < 0 || c as usize >= WIDTH { return false; }
        if r < 0 { continue; }
        if r as usize >= HEIGHT { return false; }
        if grid.cells[r as usize][c as usize] != CellState::Empty { return false; }
    }
    true
}

fn move_horizontal_vz(grid: &Grid, piece: &mut ActivePiece, delta: i32) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    let new_col = piece.col + delta;
    if is_valid_position_vz(grid, &cells, piece.row, new_col) { piece.col = new_col; true } else { false }
}

fn move_down_vz(grid: &Grid, piece: &mut ActivePiece) -> bool {
    let cells = piece_cells(piece.piece_type, piece.rotation);
    if is_valid_position_vz(grid, &cells, piece.row + 1, piece.col) { piece.row += 1; true } else { false }
}

fn rotate_vz(grid: &Grid, piece: &mut ActivePiece, clockwise: bool) -> bool {
    let new_rot = if clockwise { (piece.rotation + 1) % 4 } else { (piece.rotation + 3) % 4 };
    let cells = piece_cells(piece.piece_type, new_rot);
    if is_valid_position_vz(grid, &cells, piece.row, piece.col) {
        piece.rotation = new_rot; return true;
    }
    let kicks = srs_kicks(piece.piece_type, piece.rotation, clockwise);
    for k in &kicks {
        if is_valid_position_vz(grid, &cells, piece.row + k.1, piece.col + k.0) {
            piece.rotation = new_rot; piece.col += k.0; piece.row += k.1; return true;
        }
    }
    false
}

// --- Winit key to pipeline key ---
fn winit_to_rg(key: WinitKeyCode) -> RgKeyCode {
    match key {
        WinitKeyCode::ArrowLeft => RgKeyCode::Left,
        WinitKeyCode::ArrowRight => RgKeyCode::Right,
        WinitKeyCode::ArrowDown => RgKeyCode::Down,
        WinitKeyCode::ArrowUp => RgKeyCode::Up,
        WinitKeyCode::KeyZ => RgKeyCode::Z,
        WinitKeyCode::Space | WinitKeyCode::KeyX => RgKeyCode::Space,
        WinitKeyCode::KeyP => RgKeyCode::P,
        WinitKeyCode::Escape => RgKeyCode::Escape,
        WinitKeyCode::Enter => RgKeyCode::Enter,
        _ => RgKeyCode::Other,
    }
}

// --- Game World (wraps pipeline GameSession with vanish-zone movement) ---
struct GameWorld {
    session: GameSession,
    last_tick: Instant,
}

impl GameWorld {
    fn new() -> Self {
        GameWorld { session: GameSession::new(), last_tick: Instant::now() }
    }

    fn tick(&mut self) {
        if self.session.state != GameState::Playing { return; }
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // Use pipeline tick for gravity/locking/spawning
        self.session.gravity_accumulator_ms += (dt * 1000.0) as u64;
        let level = level_for_lines(self.session.total_lines);
        let interval = gravity_interval_ms(level);
        if self.session.gravity_accumulator_ms >= interval {
            if !move_down_vz(&self.session.grid, &mut self.session.active_piece) {
                let lines = lock_piece(&mut self.session.grid, &self.session.active_piece);
                let next_type = TETROMINO_TYPES[self.session.bag.next()];
                match try_spawn(next_type, &self.session.grid) {
                    None => { self.session.state = GameState::GameOver; }
                    Some((row, col)) => {
                        self.session.active_piece = ActivePiece {
                            piece_type: next_type, rotation: 0, row, col
                        };
                        self.session.total_lines += lines;
                        let new_level = level_for_lines(self.session.total_lines);
                        self.session.score += score_for_lines(lines, new_level);
                    }
                }
            }
            self.session.gravity_accumulator_ms = 0;
        }
    }

    fn handle_action(&mut self, action: GameAction) {
        match self.session.state {
            GameState::Playing => match action {
                GameAction::MoveLeft => { move_horizontal_vz(&self.session.grid, &mut self.session.active_piece, -1); }
                GameAction::MoveRight => { move_horizontal_vz(&self.session.grid, &mut self.session.active_piece, 1); }
                GameAction::SoftDrop => {
                    if !move_down_vz(&self.session.grid, &mut self.session.active_piece) {
                        self.lock_and_spawn();
                    }
                    self.session.gravity_accumulator_ms = 0;
                }
                GameAction::HardDrop => {
                    let lines = hard_drop(&mut self.session.grid, &self.session.active_piece);
                    self.session.total_lines += lines;
                    let level = level_for_lines(self.session.total_lines);
                    self.session.score += score_for_lines(lines, level);
                    self.spawn_or_game_over();
                    self.session.gravity_accumulator_ms = 0;
                }
                GameAction::RotateCW => { rotate_vz(&self.session.grid, &mut self.session.active_piece, true); }
                GameAction::RotateCCW => { rotate_vz(&self.session.grid, &mut self.session.active_piece, false); }
                GameAction::TogglePause => { self.session.state = GameState::Paused; }
                _ => {}
            }
            GameState::Paused => {
                if action == GameAction::TogglePause {
                    self.session.state = GameState::Playing;
                    self.last_tick = Instant::now();
                }
            }
            GameState::GameOver | GameState::Menu => {
                if action == GameAction::StartGame {
                    *self = GameWorld::new();
                }
            }
        }
    }

    fn lock_and_spawn(&mut self) {
        let s = &mut self.session;
        let lines = lock_piece(&mut s.grid, &s.active_piece);
        s.total_lines += lines;
        let level = level_for_lines(s.total_lines);
        s.score += score_for_lines(lines, level);
        self.spawn_or_game_over();
    }

    fn spawn_or_game_over(&mut self) {
        let s = &mut self.session;
        let next_type = TETROMINO_TYPES[s.bag.next()];
        match try_spawn(next_type, &s.grid) {
            None => { s.state = GameState::GameOver; }
            Some((r, c)) => {
                s.active_piece = ActivePiece { piece_type: next_type, rotation: 0, row: r, col: c };
            }
        }
    }

    fn build_vertices(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();

        let bg = rgba_to_f32([20, 20, 30, 255]);
        push_quad(&mut verts, &mut indices, 0.0, 0.0, WIN_W as f32, WIN_H as f32, bg, 0.0);

        // Sidebar
        let sidebar_bg = rgba_to_f32([30, 30, 45, 255]);
        push_quad(&mut verts, &mut indices,
            BOARD_WIDTH_PX as f32 + 1.0, 0.0, SIDEBAR_W as f32, WIN_H as f32, sidebar_bg, 0.01);

        // Grid border
        let border = rgba_to_f32([60, 60, 80, 255]);
        push_quad(&mut verts, &mut indices,
            BOARD_WIDTH_PX as f32, 0.0, 1.0, WIN_H as f32, border, 0.02);

        // Board quads from pipeline
        let quads = board_quads(&self.session.grid, &self.session.active_piece, 0, 0, CELL_SIZE);
        for (px_x, px_y, pw, ph, color) in &quads {
            let gap = 1.0;
            let fc = rgba_to_f32(*color);
            let z = if color[3] < 255 { 0.03 } else { 0.04 };
            push_quad(&mut verts, &mut indices,
                *px_x as f32 + gap, *px_y as f32 + gap,
                *pw as f32 - gap * 2.0, *ph as f32 - gap * 2.0, fc, z);
        }

        // Next piece preview
        let sx = BOARD_WIDTH_PX as f32 + 12.0;
        let text_col = rgba_to_f32([180, 180, 200, 255]);
        let dim_col = rgba_to_f32([100, 100, 120, 255]);

        push_text(&mut verts, &mut indices, sx, 10.0, "NEXT", text_col, 2.0);
        let preview_cell = 20; // smaller cells for preview to fit sidebar
        let preview_quads = next_piece_quads(
            self.session.bag.peek(), sx as i32 + 20, 35, preview_cell);
        for (px_x, px_y, pw, ph, color) in &preview_quads {
            let fc = rgba_to_f32(*color);
            push_quad(&mut verts, &mut indices,
                *px_x as f32 + 1.0, *px_y as f32 + 1.0,
                *pw as f32 - 2.0, *ph as f32 - 2.0, fc, 0.05);
        }

        // Score/Level/Lines
        push_text(&mut verts, &mut indices, sx, 140.0, "SCORE", text_col, 2.0);
        push_text(&mut verts, &mut indices, sx, 160.0,
            &format!("{}", self.session.score), text_col, 2.0);

        let level = level_for_lines(self.session.total_lines);
        push_text(&mut verts, &mut indices, sx, 195.0, "LEVEL", text_col, 2.0);
        push_text(&mut verts, &mut indices, sx, 215.0,
            &format!("{}", level), text_col, 2.0);

        push_text(&mut verts, &mut indices, sx, 250.0, "LINES", text_col, 2.0);
        push_text(&mut verts, &mut indices, sx, 270.0,
            &format!("{}", self.session.total_lines), text_col, 2.0);

        // Controls
        let cy = 320.0;
        push_text(&mut verts, &mut indices, sx, cy, "CONTROLS", dim_col, 2.0);
        push_text(&mut verts, &mut indices, sx, cy + 22.0, "L-R MOVE", dim_col, 2.0);
        push_text(&mut verts, &mut indices, sx, cy + 44.0, "DN  DROP", dim_col, 2.0);
        push_text(&mut verts, &mut indices, sx, cy + 66.0, "SPC HARD", dim_col, 2.0);
        push_text(&mut verts, &mut indices, sx, cy + 88.0, "UP  CW", dim_col, 2.0);
        push_text(&mut verts, &mut indices, sx, cy + 110.0, "Z   CCW", dim_col, 2.0);
        push_text(&mut verts, &mut indices, sx, cy + 132.0, "P  PAUSE", dim_col, 2.0);

        // State overlays
        if self.session.state == GameState::GameOver {
            let red_overlay = rgba_to_f32([200, 0, 0, 120]);
            push_quad(&mut verts, &mut indices, 0.0, 0.0,
                BOARD_WIDTH_PX as f32, BOARD_HEIGHT_PX as f32, red_overlay, 0.08);
            push_text(&mut verts, &mut indices, sx, 500.0, "GAME OVER",
                rgba_to_f32([255, 80, 80, 255]), 2.0);
            push_text(&mut verts, &mut indices, sx, 525.0, "ENTER", dim_col, 1.0);
        }

        if self.session.state == GameState::Paused {
            let dim_overlay = rgba_to_f32([0, 0, 0, 100]);
            push_quad(&mut verts, &mut indices, 0.0, 0.0,
                BOARD_WIDTH_PX as f32, BOARD_HEIGHT_PX as f32, dim_overlay, 0.08);
            push_text(&mut verts, &mut indices, sx, 500.0, "PAUSED",
                rgba_to_f32([255, 255, 100, 255]), 2.0);
            push_text(&mut verts, &mut indices, sx, 525.0, "P", dim_col, 1.0);
        }

        (verts, indices)
    }
}

// --- GPU State ---
struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
}

impl GpuState {
    fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        })).expect("request adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(), None,
        )).expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        GpuState { surface, device, queue, config, pipeline }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&self, verts: &[Vertex], indices: &[u32]) {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = output.texture.create_view(&Default::default());

        let vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.08, g: 0.08, b: 0.12, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

// --- Winit App ---
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    world: GameWorld,
    pending_resize: Option<(u32, u32)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes()
            .with_title("RhythmGrid")
            .with_inner_size(winit::dpi::LogicalSize::new(WIN_W, WIN_H))
            .with_min_inner_size(winit::dpi::LogicalSize::new(WIN_W / 2, WIN_H / 2));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let gpu = GpuState::new(window.clone());
        self.gpu = Some(gpu);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                self.pending_resize = Some((new_size.width, new_size.height));
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(code), state: ElementState::Pressed, .. }, .. } => {
                let rg_key = winit_to_rg(code);
                if let Some(action) = input::map_key(rg_key) {
                    self.world.handle_action(action);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some((w, h)) = self.pending_resize.take() {
                    if let Some(gpu) = &mut self.gpu {
                        gpu.resize(w, h);
                    }
                }
                self.world.tick();
                let (verts, indices) = self.world.build_vertices();
                if let Some(gpu) = &self.gpu {
                    gpu.render(&verts, &indices);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App {
        window: None,
        gpu: None,
        world: GameWorld::new(),
        pending_resize: None,
    };
    event_loop.run_app(&mut app).expect("event loop");
}
