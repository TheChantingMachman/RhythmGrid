// Drawing primitives — quads, 3D blocks, panels, text.

use super::font::FONT;
use super::theme::THEME;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub fn rgba_to_f32(c: [u8; 4]) -> [f32; 4] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0]
}

pub fn darken(c: [f32; 4], factor: f32) -> [f32; 4] {
    [c[0] * factor, c[1] * factor, c[2] * factor, c[3]]
}

pub fn brighten(c: [f32; 4], factor: f32) -> [f32; 4] {
    [(c[0] * factor).min(1.0), (c[1] * factor).min(1.0), (c[2] * factor).min(1.0), c[3]]
}

fn px_to_ndc(px_x: f32, px_y: f32, win_w: f32, win_h: f32) -> (f32, f32) {
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
    verts.push(Vertex { position: [x0, y0, z], color });
    verts.push(Vertex { position: [x1, y0, z], color });
    verts.push(Vertex { position: [x1, y1, z], color });
    verts.push(Vertex { position: [x0, y1, z], color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

pub fn push_block_cam(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                      px_x: f32, px_y: f32, size: f32, color: [f32; 4], depth: f32, z_order: f32,
                      iso_dx: f32, iso_dy: f32) {
    let ww = THEME.win_w as f32;
    let wh = THEME.win_h as f32;
    let gap = 1.0;
    let x = px_x + gap;
    let y = px_y + gap;
    let s = size - gap * 2.0;
    let dx = depth * iso_dx;
    let dy = depth * iso_dy;

    // Neon glow — soft halo behind the block in its own color
    let glow_spread = 3.0;
    let glow_color = [color[0], color[1], color[2], color[3] * 0.15];
    let (gx0, gy0) = px_to_ndc(x - glow_spread, y - glow_spread, ww, wh);
    let (gx1, gy1) = px_to_ndc(x + s + glow_spread, y + s + glow_spread, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [gx0, gy0, z_order - 0.003], color: glow_color });
    verts.push(Vertex { position: [gx1, gy0, z_order - 0.003], color: glow_color });
    verts.push(Vertex { position: [gx1, gy1, z_order - 0.003], color: glow_color });
    verts.push(Vertex { position: [gx0, gy1, z_order - 0.003], color: glow_color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Front face (bright core)
    let front = brighten(color, 1.1);
    let (x0, y0) = px_to_ndc(x, y, ww, wh);
    let (x1, y1) = px_to_ndc(x + s, y + s, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x0, y0, z_order], color: front });
    verts.push(Vertex { position: [x1, y0, z_order], color: front });
    verts.push(Vertex { position: [x1, y1, z_order], color: front });
    verts.push(Vertex { position: [x0, y1, z_order], color: front });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Inner highlight (hot center of neon tube)
    let highlight = brighten(color, 1.6);
    let inset = s * 0.2;
    let (hx0, hy0) = px_to_ndc(x + inset, y + inset, ww, wh);
    let (hx1, hy1) = px_to_ndc(x + s - inset, y + s - inset, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [hx0, hy0, z_order + 0.001], color: highlight });
    verts.push(Vertex { position: [hx1, hy0, z_order + 0.001], color: highlight });
    verts.push(Vertex { position: [hx1, hy1, z_order + 0.001], color: highlight });
    verts.push(Vertex { position: [hx0, hy1, z_order + 0.001], color: highlight });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Top face (brighter)
    let top = brighten(color, 1.15);
    let (tx0, ty0) = px_to_ndc(x + dx, y + dy, ww, wh);
    let (tx1, ty1) = px_to_ndc(x + s + dx, y + dy, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [tx0, ty0, z_order - 0.001], color: top });
    verts.push(Vertex { position: [tx1, ty1, z_order - 0.001], color: top });
    verts.push(Vertex { position: [x1, y0, z_order - 0.001], color: top });
    verts.push(Vertex { position: [x0, y0, z_order - 0.001], color: top });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Side face (darker) — adapts to camera direction
    let side = darken(color, 0.6);
    let (sx, sy) = if iso_dx >= 0.0 {
        // Camera from right → show right side
        (x + s, y)
    } else {
        // Camera from left → show left side
        (x, y)
    };
    let (rx0, ry0) = px_to_ndc(sx, sy, ww, wh);
    let (rx1, ry1) = px_to_ndc(sx + dx, sy + dy, ww, wh);
    let (rx2, ry2) = px_to_ndc(sx + dx, sy + s + dy, ww, wh);
    let (rx3, ry3) = px_to_ndc(sx, sy + s, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [rx0, ry0, z_order - 0.001], color: side });
    verts.push(Vertex { position: [rx1, ry1, z_order - 0.001], color: side });
    verts.push(Vertex { position: [rx2, ry2, z_order - 0.001], color: side });
    verts.push(Vertex { position: [rx3, ry3, z_order - 0.001], color: side });
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
                            scale, scale, color, 0.0);
                    }
                }
            }
        }
    }
}
