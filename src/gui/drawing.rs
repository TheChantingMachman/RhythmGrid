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

    // Rounded cube: main faces inset, multi-strip bevel for smooth edges
    let b = 0.05; // total bevel size
    let steps = 3u32; // bevel subdivisions for smoothness

    // Inner bounds (inset by bevel)
    let ix0 = x0 + b;
    let ix1 = x1 - b;
    let iy0 = y0 - b;
    let iy1 = y1 + b;
    let iz1 = z1 - b;

    // Main faces (inset)
    let faces: &[([f32; 3], [[f32; 3]; 4])] = &[
        ([0.0, 0.0, 1.0],  [[ix0, iy0, z1], [ix1, iy0, z1], [ix1, iy1, z1], [ix0, iy1, z1]]),  // Front
        ([0.0, 0.0, -1.0], [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]]),          // Back
        ([0.0, 1.0, 0.0],  [[ix0, y0, iz1], [ix0, y0, z0], [ix1, y0, z0], [ix1, y0, iz1]]),     // Top
        ([0.0, -1.0, 0.0], [[ix0, y1, z0], [ix0, y1, iz1], [ix1, y1, iz1], [ix1, y1, z0]]),     // Bottom
        ([1.0, 0.0, 0.0],  [[x1, iy0, iz1], [x1, iy0, z0], [x1, iy1, z0], [x1, iy1, iz1]]),    // Right
        ([-1.0, 0.0, 0.0], [[x0, iy0, z0], [x0, iy0, iz1], [x0, iy1, iz1], [x0, iy1, z0]]),    // Left
    ];

    // Per-vertex edge glow: edges bright (original color + white),
    // face centers darker. Creates beveled gemstone look.
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let cz = (z0 + z1) * 0.5;

    for (normal, corners) in faces {
        let base_idx = verts.len() as u32;
        for &pos in corners {
            let dx = (pos[0] - cx).abs() / ((x1 - x0) * 0.5);
            let dy = (pos[1] - cy).abs() / ((y0 - y1) * 0.5);
            let dz = (pos[2] - cz).abs() / ((z1 - z0) * 0.5);
            let edge_factor = match normal {
                [_, _, z] if z.abs() > 0.5 => (dx + dy) * 0.5,
                [_, y, _] if y.abs() > 0.5 => (dx + dz) * 0.5,
                _                           => (dy + dz) * 0.5,
            };
            // Edge highlight + fake subsurface on back face
            let highlight = edge_factor * 0.08;
            let is_back = normal[2] < -0.5;
            let (r, g, b, a) = if is_back {
                // Subsurface: slightly brighter — scales with darkness of color
                let lum = color[0] * 0.2126 + color[1] * 0.7152 + color[2] * 0.0722;
                let boost = 1.15 + (1.0 - lum) * 0.2; // darker colors get more boost
                let white = (1.0 - lum) * 0.08; // less white shift for bright colors
                ((color[0] * boost + white).min(1.0),
                 (color[1] * boost + white).min(1.0),
                 (color[2] * boost + white).min(1.0),
                 color[3])
            } else {
                ((color[0] + highlight).min(1.0),
                 (color[1] + highlight).min(1.0),
                 (color[2] + highlight).min(1.0),
                 color[3])
            };
            let vc = [r, g, b, a];
            verts.push(Vertex { position: pos, normal: *normal, color: vc });
        }
        indices.extend_from_slice(&[base_idx, base_idx+1, base_idx+2, base_idx, base_idx+2, base_idx+3]);
    }

    // Multi-strip bevels along front edges — smooth normal transition
    // Helper: emit a bevel strip quad with interpolated normal
    let emit_bevel = |verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                      p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3],
                      n: [f32; 3]| {
        let base_idx = verts.len() as u32;
        verts.push(Vertex { position: p0, normal: n, color });
        verts.push(Vertex { position: p1, normal: n, color });
        verts.push(Vertex { position: p2, normal: n, color });
        verts.push(Vertex { position: p3, normal: n, color });
        indices.extend_from_slice(&[base_idx, base_idx+1, base_idx+2, base_idx, base_idx+2, base_idx+3]);
    };

    // Generate bevel strips for front edges
    let pi_half = std::f32::consts::FRAC_PI_2;
    for s in 0..steps {
        let t0 = s as f32 / steps as f32;
        let t1 = (s + 1) as f32 / steps as f32;
        let a0 = t0 * pi_half;
        let a1 = t1 * pi_half;
        // Position interpolation along the bevel curve
        let (sin0, cos0) = (a0.sin(), a0.cos());
        let (sin1, cos1) = (a1.sin(), a1.cos());
        // Normal at midpoint of this strip
        let am = (a0 + a1) * 0.5;
        let (sinm, cosm) = (am.sin(), am.cos());

        // Front-top bevel: normal transitions from (0,0,1) to (0,1,0)
        let ftz0 = z1 - b + b * sin0;
        let ftz1 = z1 - b + b * sin1;
        let fty0 = y0 - b + b * cos0;
        let fty1 = y0 - b + b * cos1;
        emit_bevel(verts, indices,
            [ix0, fty0, ftz0], [ix1, fty0, ftz0], [ix1, fty1, ftz1], [ix0, fty1, ftz1],
            [0.0, cosm, sinm]);

        // Front-bottom bevel: normal transitions from (0,0,1) to (0,-1,0)
        let fby0 = y1 + b - b * cos0;
        let fby1 = y1 + b - b * cos1;
        emit_bevel(verts, indices,
            [ix0, fby1, ftz1], [ix1, fby1, ftz1], [ix1, fby0, ftz0], [ix0, fby0, ftz0],
            [0.0, -cosm, sinm]);

        // Front-right bevel: normal transitions from (0,0,1) to (1,0,0)
        let frx0 = x1 - b + b * sin0;
        let frx1 = x1 - b + b * sin1;
        let frz0 = z1 - b + b * cos0;  // note: cos for z here
        let frz1 = z1 - b + b * cos1;
        emit_bevel(verts, indices,
            [frx0, iy0, frz0], [frx1, iy0, frz1], [frx1, iy1, frz1], [frx0, iy1, frz0],
            [sinm, 0.0, cosm]);

        // Front-left bevel
        let flx0 = x0 + b - b * sin0;
        let flx1 = x0 + b - b * sin1;
        emit_bevel(verts, indices,
            [flx1, iy0, frz1], [flx0, iy0, frz0], [flx0, iy1, frz0], [flx1, iy1, frz1],
            [-sinm, 0.0, cosm]);
    }
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

/// Extrude a 2D polygon into a 3D shape with depth. Points are in world-space XY
/// (y-down convention: y is negated internally). z0=back, z1=front.
pub fn push_extruded_shape(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                           points: &[[f32; 2]], z0: f32, z1: f32, color: [f32; 4]) {
    let n = points.len();
    if n < 3 { return; }

    // Front face (z1) — fan triangulation
    let front_n = [0.0f32, 0.0, 1.0];
    let base = verts.len() as u32;
    for &[x, y] in points {
        verts.push(Vertex { position: [x, -y, z1], normal: front_n, color });
    }
    for i in 1..n as u32 - 1 {
        indices.extend_from_slice(&[base, base + i, base + i + 1]);
    }

    // Back face (z0) — reversed winding
    let back_n = [0.0f32, 0.0, -1.0];
    let base = verts.len() as u32;
    for &[x, y] in points {
        verts.push(Vertex { position: [x, -y, z0], normal: back_n, color });
    }
    for i in 1..n as u32 - 1 {
        indices.extend_from_slice(&[base, base + i + 1, base + i]);
    }

    // Side faces — quads connecting front and back edges
    for i in 0..n {
        let j = (i + 1) % n;
        let [x0, y0] = points[i];
        let [x1, y1] = points[j];
        // Edge normal (perpendicular to edge, pointing outward)
        let dx = x1 - x0;
        let dy = -(y1 - y0); // negate because y is flipped
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let side_n = [dy / len, dx / len, 0.0]; // perpendicular in XY

        let base = verts.len() as u32;
        verts.push(Vertex { position: [x0, -y0, z1], normal: side_n, color });
        verts.push(Vertex { position: [x1, -y1, z1], normal: side_n, color });
        verts.push(Vertex { position: [x1, -y1, z0], normal: side_n, color });
        verts.push(Vertex { position: [x0, -y0, z0], normal: side_n, color });
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }
}

pub fn push_grid_line_v(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                        x: f32, height: f32, color: [f32; 4], thickness: f32) {
    let w = thickness;
    let n = [0.0f32, 0.0, 1.0];
    let base = verts.len() as u32;
    verts.push(Vertex { position: [x - w, 0.0, 0.0], normal: n, color });
    verts.push(Vertex { position: [x + w, 0.0, 0.0], normal: n, color });
    verts.push(Vertex { position: [x + w, -height, 0.0], normal: n, color });
    verts.push(Vertex { position: [x - w, -height, 0.0], normal: n, color });
    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
}

pub fn push_grid_line_h(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                        y: f32, width: f32, color: [f32; 4], thickness: f32) {
    let w = thickness;
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
