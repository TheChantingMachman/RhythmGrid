// Drawing primitives — quads, 3D blocks, panels, text.

use super::font::FONT;
use super::theme::THEME;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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
    verts.push(Vertex { position: [x0, y0, z], color });
    verts.push(Vertex { position: [x1, y0, z], color });
    verts.push(Vertex { position: [x1, y1, z], color });
    verts.push(Vertex { position: [x0, y1, z], color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

/// High-fidelity block with gradient faces, edge bevel, and specular highlight.
/// `glow_boost` (0.0-1.0+) amplifies the outer glow (for amplitude reactivity).
pub fn push_block_cam(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                      px_x: f32, px_y: f32, size: f32, color: [f32; 4], depth: f32, z_order: f32,
                      iso_dx: f32, iso_dy: f32) {
    push_block_ex(verts, indices, px_x, px_y, size, color, depth, z_order, iso_dx, iso_dy, 0.0);
}

pub fn push_block_ex(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                     px_x: f32, px_y: f32, size: f32, color: [f32; 4], depth: f32, z_order: f32,
                     iso_dx: f32, iso_dy: f32, glow_boost: f32) {
    let ww = THEME.win_w as f32;
    let wh = THEME.win_h as f32;
    let gap = 1.0;
    let x = px_x + gap;
    let y = px_y + gap;
    let s = size - gap * 2.0;
    let dx = depth * iso_dx;
    let dy = depth * iso_dy;

    // --- Multi-layer bloom glow (fake bloom via stacked radial fades) ---
    let layers: [(f32, f32); 3] = [
        (12.0 + glow_boost * 8.0, 0.04 + glow_boost * 0.03),  // outer soft
        (7.0 + glow_boost * 5.0,  0.07 + glow_boost * 0.06),  // mid
        (4.0 + glow_boost * 3.0,  0.12 + glow_boost * 0.10),  // inner bright
    ];
    let cx = x + s * 0.5;
    let cy = y + s * 0.5;
    for (spread, alpha) in &layers {
        let a = alpha.min(0.5);
        let gc = [color[0], color[1], color[2], color[3] * a];
        let ge = [color[0] * 0.3, color[1] * 0.3, color[2] * 0.3, 0.0];
        let (gx0, gy0) = px_to_ndc(x - spread, y - spread, ww, wh);
        let (gx1, gy1) = px_to_ndc(x + s + spread, y + s + spread, ww, wh);
        let (gmx, gmy) = px_to_ndc(cx, cy, ww, wh);
        let base = verts.len() as u32;
        verts.push(Vertex { position: [gmx, gmy, z_order - 0.004], color: gc });
        verts.push(Vertex { position: [gx0, gy0, z_order - 0.004], color: ge });
        verts.push(Vertex { position: [gx1, gy0, z_order - 0.004], color: ge });
        verts.push(Vertex { position: [gx1, gy1, z_order - 0.004], color: ge });
        verts.push(Vertex { position: [gx0, gy1, z_order - 0.004], color: ge });
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3, base, base+3, base+4, base, base+4, base+1]);
    }

    // --- Front face with per-vertex gradient (beveled look) ---
    let edge_dark = darken(color, 0.5);
    let face_bright = brighten(color, 1.15);
    let bevel = s * 0.12; // bevel inset

    let (x0, y0) = px_to_ndc(x, y, ww, wh);
    let (x1, y1) = px_to_ndc(x + s, y + s, ww, wh);
    let (ix0, iy0) = px_to_ndc(x + bevel, y + bevel, ww, wh);
    let (ix1, iy1) = px_to_ndc(x + s - bevel, y + s - bevel, ww, wh);

    // Outer edge ring (4 quads forming bevel)
    // Top bevel
    let base = verts.len() as u32;
    let top_edge = brighten(color, 0.9);
    verts.push(Vertex { position: [x0, y0, z_order], color: edge_dark });
    verts.push(Vertex { position: [x1, y0, z_order], color: edge_dark });
    verts.push(Vertex { position: [ix1, iy0, z_order], color: top_edge });
    verts.push(Vertex { position: [ix0, iy0, z_order], color: top_edge });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Bottom bevel
    let base = verts.len() as u32;
    verts.push(Vertex { position: [ix0, iy1, z_order], color: darken(color, 0.7) });
    verts.push(Vertex { position: [ix1, iy1, z_order], color: darken(color, 0.7) });
    verts.push(Vertex { position: [x1, y1, z_order], color: edge_dark });
    verts.push(Vertex { position: [x0, y1, z_order], color: edge_dark });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Left bevel
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x0, y0, z_order], color: edge_dark });
    verts.push(Vertex { position: [ix0, iy0, z_order], color: darken(color, 0.8) });
    verts.push(Vertex { position: [ix0, iy1, z_order], color: darken(color, 0.7) });
    verts.push(Vertex { position: [x0, y1, z_order], color: edge_dark });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Right bevel
    let base = verts.len() as u32;
    verts.push(Vertex { position: [ix1, iy0, z_order], color: brighten(color, 0.85) });
    verts.push(Vertex { position: [x1, y0, z_order], color: edge_dark });
    verts.push(Vertex { position: [x1, y1, z_order], color: edge_dark });
    verts.push(Vertex { position: [ix1, iy1, z_order], color: darken(color, 0.65) });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Inner face (gradient: bright top-left to darker bottom-right)
    let base = verts.len() as u32;
    let tl = brighten(face_bright, 1.1);  // top-left: brightest (specular)
    let tr = face_bright;
    let br = darken(color, 0.8);           // bottom-right: darkest
    let bl = darken(color, 0.9);
    verts.push(Vertex { position: [ix0, iy0, z_order + 0.001], color: tl });
    verts.push(Vertex { position: [ix1, iy0, z_order + 0.001], color: tr });
    verts.push(Vertex { position: [ix1, iy1, z_order + 0.001], color: br });
    verts.push(Vertex { position: [ix0, iy1, z_order + 0.001], color: bl });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // Specular highlight dot (upper-left of inner face)
    let spec_size = s * 0.15;
    let spec_color = [1.0f32.min(color[0] + 0.5), 1.0f32.min(color[1] + 0.5), 1.0f32.min(color[2] + 0.5), color[3] * 0.6];
    let spec_edge = [spec_color[0], spec_color[1], spec_color[2], 0.0];
    let (sx0, sy0) = px_to_ndc(x + bevel + spec_size * 0.5, y + bevel + spec_size * 0.5, ww, wh);
    let (sx1, sy1) = px_to_ndc(x + bevel + spec_size * 2.5, y + bevel + spec_size * 2.5, ww, wh);
    let (smx, smy) = px_to_ndc(x + bevel + spec_size, y + bevel + spec_size, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [smx, smy, z_order + 0.002], color: spec_color });
    verts.push(Vertex { position: [sx0, sy0, z_order + 0.002], color: spec_edge });
    verts.push(Vertex { position: [sx1, sy0, z_order + 0.002], color: spec_edge });
    verts.push(Vertex { position: [sx1, sy1, z_order + 0.002], color: spec_edge });
    verts.push(Vertex { position: [sx0, sy1, z_order + 0.002], color: spec_edge });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3, base, base+3, base+4, base, base+4, base+1]);

    // --- Top face (iso extrusion, gradient light→dark) ---
    let top_lit = brighten(color, 1.2);
    let top_dark = brighten(color, 0.7);
    let (tx0, ty0) = px_to_ndc(x + dx, y + dy, ww, wh);
    let (tx1, ty1) = px_to_ndc(x + s + dx, y + dy, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [tx0, ty0, z_order - 0.001], color: top_lit });
    verts.push(Vertex { position: [tx1, ty1, z_order - 0.001], color: top_lit });
    verts.push(Vertex { position: [x1, y0, z_order - 0.001], color: top_dark });
    verts.push(Vertex { position: [x0, y0, z_order - 0.001], color: top_dark });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

    // --- Side face (darker, gradient) ---
    let side_lit = darken(color, 0.55);
    let side_dark = darken(color, 0.35);
    let (side_x, side_y) = if iso_dx >= 0.0 { (x + s, y) } else { (x, y) };
    let (rx0, ry0) = px_to_ndc(side_x, side_y, ww, wh);
    let (rx1, ry1) = px_to_ndc(side_x + dx, side_y + dy, ww, wh);
    let (rx2, ry2) = px_to_ndc(side_x + dx, side_y + s + dy, ww, wh);
    let (rx3, ry3) = px_to_ndc(side_x, side_y + s, ww, wh);
    let base = verts.len() as u32;
    verts.push(Vertex { position: [rx0, ry0, z_order - 0.001], color: side_lit });
    verts.push(Vertex { position: [rx1, ry1, z_order - 0.001], color: side_dark });
    verts.push(Vertex { position: [rx2, ry2, z_order - 0.001], color: side_dark });
    verts.push(Vertex { position: [rx3, ry3, z_order - 0.001], color: side_lit });
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
/// Each face has uniform color with lighting baked in via face direction.
pub fn push_cube_3d(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                    col: f32, row: f32, depth: f32, color: [f32; 4],
                    glow_boost: f32) {
    let gap = 0.04;
    let x0 = col + gap;
    let x1 = col + 1.0 - gap;
    let y0 = -row - gap;        // top of cell (y-up)
    let y1 = -row - 1.0 + gap;  // bottom of cell
    let z0 = 0.0;               // back face
    let z1 = depth;              // front face (toward camera)

    // Face colors: front bright, top brighter, sides darker, bottom darkest
    let front = brighten(color, 1.1);
    let back = darken(color, 0.3);
    let top = brighten(color, 1.25);
    let bottom = darken(color, 0.5);
    let right = darken(color, 0.65);
    let left = darken(color, 0.75);

    let faces: &[([f32; 4], [[f32; 3]; 4])] = &[
        // Front (z1) — CCW from front
        (front, [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]]),
        // Back (z0)
        (back, [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]]),
        // Top (y0)
        (top, [[x0, y0, z1], [x0, y0, z0], [x1, y0, z0], [x1, y0, z1]]),
        // Bottom (y1)
        (bottom, [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]]),
        // Right (x1)
        (right, [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]]),
        // Left (x0)
        (left, [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]]),
    ];

    for (face_color, corners) in faces {
        let base = verts.len() as u32;
        for &pos in corners {
            verts.push(Vertex { position: pos, color: *face_color });
        }
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }

    // Glow quad behind the front face (for neon effect)
    if glow_boost > 0.01 || color[3] > 0.5 {
        let ga = (0.08 + glow_boost * 0.15).min(0.4);
        let gc = [color[0], color[1], color[2], color[3] * ga];
        let spread = 0.15 + glow_boost * 0.1;
        let base = verts.len() as u32;
        verts.push(Vertex { position: [x0 - spread, y0 + spread, z1 + 0.01], color: gc });
        verts.push(Vertex { position: [x1 + spread, y0 + spread, z1 + 0.01], color: gc });
        verts.push(Vertex { position: [x1 + spread, y1 - spread, z1 + 0.01], color: gc });
        verts.push(Vertex { position: [x0 - spread, y1 - spread, z1 + 0.01], color: gc });
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }
}

/// 3D grid floor quad in world space
pub fn push_grid_floor(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                       width: f32, height: f32, color: [f32; 4]) {
    let base = verts.len() as u32;
    verts.push(Vertex { position: [0.0, 0.0, -0.01], color });
    verts.push(Vertex { position: [width, 0.0, -0.01], color });
    verts.push(Vertex { position: [width, -height, -0.01], color });
    verts.push(Vertex { position: [0.0, -height, -0.01], color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

/// 3D grid line in world space
pub fn push_grid_line_v(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                        x: f32, height: f32, color: [f32; 4]) {
    let w = 0.02;
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x - w, 0.0, 0.0], color });
    verts.push(Vertex { position: [x + w, 0.0, 0.0], color });
    verts.push(Vertex { position: [x + w, -height, 0.0], color });
    verts.push(Vertex { position: [x - w, -height, 0.0], color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

pub fn push_grid_line_h(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                        y: f32, width: f32, color: [f32; 4]) {
    let w = 0.02;
    let base = verts.len() as u32;
    verts.push(Vertex { position: [0.0, y - w, 0.0], color });
    verts.push(Vertex { position: [width, y - w, 0.0], color });
    verts.push(Vertex { position: [width, y + w, 0.0], color });
    verts.push(Vertex { position: [0.0, y + w, 0.0], color });
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
