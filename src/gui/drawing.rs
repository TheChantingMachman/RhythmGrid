// Drawing primitives — quads, 3D blocks, panels, text.

use super::font::FONT;
use super::theme::THEME;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,  // position
        1 => Float32x3,  // normal
        2 => Float32x4,  // color
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Default normal for 2D HUD elements (facing camera)
pub const HUD_NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

pub fn rgba_to_f32(c: [u8; 4]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0]
}


pub fn px_to_ndc(px_x: f32, px_y: f32, win_w: f32, win_h: f32) -> (f32, f32) {
    let nx = (px_x / win_w) * 2.0 - 1.0;
    let ny = 1.0 - (px_y / win_h) * 2.0;
    (nx, ny)
}

pub fn push_quad(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                 x: f32, y: f32, w: f32, h: f32, color: [f32; 4], z: f32) {
    let ww = THEME.win_w as f32;
    let wh = THEME.win_h as f32;
    let (x0, y0) = px_to_ndc(x, y, ww, wh);
    let (x1, y1) = px_to_ndc(x + w, y + h, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x0, y0, z], normal: HUD_NORMAL, color });
    verts.push(Vertex { position: [x1, y0, z], normal: HUD_NORMAL, color });
    verts.push(Vertex { position: [x1, y1, z], normal: HUD_NORMAL, color });
    verts.push(Vertex { position: [x0, y1, z], normal: HUD_NORMAL, color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}



pub fn push_panel(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                  x: f32, y: f32, w: f32, h: f32, z: f32) {
    let bg = rgba_to_f32(THEME.panel_bg);
    let border = rgba_to_f32(THEME.panel_border);
    let bw = 1.0;
    push_quad(verts, indices, x - bw, y - bw, w + bw * 2.0, h + bw * 2.0, border, z - 0.001);
    push_quad(verts, indices, x, y, w, h, bg, z);
    let highlight = rgba_to_f32([60, 60, 90, 100]);
    push_quad(verts, indices, x, y, w, 1.0, highlight, z + 0.001);
}

/// 3D cube in world space. Position is grid (col, row), y-up convention.
/// `glow_boost` (typically amplitude * 2.0) modulates color saturation and brightness.
pub fn push_cube_3d(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                    col: f32, row: f32, depth: f32, color: [f32; 4],
                    glow_boost: f32) {
    let gap = 0.08; // visible gap between cubes creates grid structure
    let x0 = col + gap;
    let x1 = col + 1.0 - gap;
    let y0 = -row - gap;        // top of cell (y-up)
    let y1 = -row - 1.0 + gap;  // bottom of cell
    let z0 = -depth * 0.5;      // back face (behind grid plane)
    let z1 = depth * 0.5;       // front face (in front of grid plane)

    // Modulate color with amplitude: quiet = desaturated, loud = vivid + bright
    let saturation = 0.8 + glow_boost * 0.2; // 0.8 base, up to 1.0+ when loud
    let brightness = 1.0 + glow_boost * 0.15;
    // Desaturate by mixing toward luminance
    let lum = color[0] * 0.2126 + color[1] * 0.7152 + color[2] * 0.0722;
    let color = [
        ((lum + (color[0] - lum) * saturation) * brightness).min(1.0),
        ((lum + (color[1] - lum) * saturation) * brightness).min(1.0),
        ((lum + (color[2] - lum) * saturation) * brightness).min(1.0),
        color[3],
    ];

    let faces: &[([f32; 3], [[f32; 3]; 4])] = &[
        // (normal, [corners])
        ([0.0, 0.0, 1.0],  [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]]),  // Front
        ([0.0, 0.0, -1.0], [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]]),  // Back
        ([0.0, 1.0, 0.0],  [[x0, y0, z1], [x0, y0, z0], [x1, y0, z0], [x1, y0, z1]]),  // Top
        ([0.0, -1.0, 0.0], [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]]),  // Bottom
        ([1.0, 0.0, 0.0],  [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]]),  // Right
        ([-1.0, 0.0, 0.0], [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]]),  // Left
    ];

    for (normal, corners) in faces {
        let base = verts.len() as u32;
        for &pos in corners {
            verts.push(Vertex { position: pos, normal: *normal, color });
        }
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }

    // Glow removed — bloom post-processing handles the soft glow now
}

/// 3D slab in world space — simplified box for dashboard elements.
/// Position is (x, y) in grid coords, y-down convention same as push_cube_3d.
pub fn push_slab_3d(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                    x: f32, y: f32, w: f32, h: f32, depth: f32, color: [f32; 4]) {
    let x0 = x;
    let x1 = x + w;
    let y0 = -y;
    let y1 = -(y + h);
    let z0 = 0.0;
    let z1 = depth;

    let faces: &[([f32; 3], [[f32; 3]; 4])] = &[
        ([0.0, 0.0, 1.0],  [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]]),
        ([0.0, 0.0, -1.0], [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]]),
        ([0.0, 1.0, 0.0],  [[x0, y0, z1], [x0, y0, z0], [x1, y0, z0], [x1, y0, z1]]),
        ([0.0, -1.0, 0.0], [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]]),
        ([1.0, 0.0, 0.0],  [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]]),
        ([-1.0, 0.0, 0.0], [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]]),
    ];
    for (normal, corners) in faces {
        let base = verts.len() as u32;
        for &pos in corners {
            verts.push(Vertex { position: pos, normal: *normal, color });
        }
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }
}

pub fn push_grid_line_v(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                        x: f32, height: f32, color: [f32; 4]) {
    let w = 0.02;
    let n = [0.0f32, 0.0, 1.0];
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x - w, 0.0, 0.0], normal: n, color });
    verts.push(Vertex { position: [x + w, 0.0, 0.0], normal: n, color });
    verts.push(Vertex { position: [x + w, -height, 0.0], normal: n, color });
    verts.push(Vertex { position: [x - w, -height, 0.0], normal: n, color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

pub fn push_grid_line_h(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                        y: f32, width: f32, color: [f32; 4]) {
    let w = 0.02;
    let n = [0.0f32, 0.0, 1.0];
    let base = verts.len() as u32;
    verts.push(Vertex { position: [0.0, y - w, 0.0], normal: n, color });
    verts.push(Vertex { position: [width, y - w, 0.0], normal: n, color });
    verts.push(Vertex { position: [width, y + w, 0.0], normal: n, color });
    verts.push(Vertex { position: [0.0, y + w, 0.0], normal: n, color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

pub fn push_text(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                 x: f32, y: f32, text: &str, color: [f32; 4], scale: f32) {
    for (i, ch) in text.chars().enumerate() {
        let upper = ch.to_ascii_uppercase();
        if let Some((_, glyph)) = FONT.iter().find(|(c, _)| *c == upper) {
            let cx = x + i as f32 * 4.0 * scale;
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..3 {
                    if bits & (1 << (2 - col)) != 0 {
                        push_quad(verts, indices,
                            cx + col as f32 * scale, y + row as f32 * scale,
                            scale, scale, color, 0.09);
                    }
                }
            }
        }
    }
}
