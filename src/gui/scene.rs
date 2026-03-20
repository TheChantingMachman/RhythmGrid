// Scene building — constructs 3D geometry and 2D HUD from game state.
// Separated from world.rs to keep rendering logic isolated from game logic.

use rhythm_grid::game::*;
use rhythm_grid::grid::*;
use rhythm_grid::pieces::*;
use rhythm_grid::render::*;

use super::drawing::*;
use super::theme::*;
use super::world::GameWorld;

/// Build 3D scene (world-space cubes, background) and 2D HUD (NDC overlay)
pub fn build_scene_and_hud(world: &GameWorld) -> ((Vec<Vertex>, Vec<u32>), (Vec<Vertex>, Vec<u32>)) {
    let amp = world.amplitude;
    let beat = world.beat_intensity;
    let cube_depth = 0.75; // chunkier cubes for more substantial 3D feel

    let mut sv = Vec::new();
    let mut si = Vec::new();

    let gw = WIDTH as f32;
    let gh = HEIGHT as f32;

    // Background geometry
    build_background(&mut sv, &mut si, world, gw, gh);

    // Grid floor
    let floor_color = rgba_to_f32([5, 5, 12, 200]);
    push_grid_floor(&mut sv, &mut si, gw, gh, floor_color);

    // Grid lines — beat + highs drive shimmer
    let line_boost = (beat * 40.0) as u8;
    let highs_boost = (world.highs * 60.0) as u8;
    let lc: [u8; 4] = [40, 45, 70, 255];
    let line_color = rgba_to_f32([
        lc[0].saturating_add(line_boost).saturating_add(highs_boost / 3),
        lc[1].saturating_add(line_boost).saturating_add(highs_boost / 2),
        lc[2].saturating_add(line_boost * 2).saturating_add(highs_boost),
        lc[3],
    ]);
    for col in 0..=WIDTH {
        push_grid_line_v(&mut sv, &mut si, col as f32, gh, line_color);
    }
    for row in 0..=HEIGHT {
        push_grid_line_h(&mut sv, &mut si, -(row as f32), gw, line_color);
    }

    // Occupied cells as 3D cubes
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            if let CellState::Occupied(ti) = world.session.grid.cells[row][col] {
                let color = rgba_to_f32(piece_color(ti));
                push_cube_3d(&mut sv, &mut si, col as f32, row as f32, cube_depth, color, amp * 2.0);
            }
        }
    }

    // Ghost piece
    if world.session.state == GameState::Playing {
        let cells = piece_cells(world.session.active_piece.piece_type, world.session.active_piece.rotation);
        let mut ghost_row = world.session.active_piece.row;
        while is_valid_position(&world.session.grid, &cells, ghost_row + 1, world.session.active_piece.col) {
            ghost_row += 1;
        }
        let base_color = piece_color(world.session.active_piece.piece_type as u32);
        let ghost_color = rgba_to_f32([base_color[0], base_color[1], base_color[2], 40]);
        for &(dr, dc) in &cells {
            let r = ghost_row + dr;
            let c = world.session.active_piece.col + dc;
            if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                push_cube_3d(&mut sv, &mut si, c as f32, r as f32, cube_depth * 0.2, ghost_color, 0.0);
            }
        }

        // Active piece
        let color = rgba_to_f32(piece_color(world.session.active_piece.piece_type as u32));
        for &(dr, dc) in &cells {
            let r = world.session.active_piece.row + dr;
            let c = world.session.active_piece.col + dc;
            if r >= 0 && c >= 0 && (r as usize) < HEIGHT && (c as usize) < WIDTH {
                push_cube_3d(&mut sv, &mut si, c as f32, r as f32, cube_depth, color, amp * 2.0);
            }
        }
    }

    // Per-cell clearing animations (shrinking bright cubes)
    for cell in &world.clearing_cells {
        if cell.scale > 0.01 {
            // Stay white throughout, fade alpha
            let progress = 1.0 - (cell.timer / super::world::LINE_CLEAR_DURATION).max(0.0);
            let alpha = (1.0 - progress).max(0.0);
            let bright_color = [1.0, 1.0, 1.0, alpha];

            // Render as a scaled cube centered on the cell
            let cx = cell.col as f32 + 0.5;
            let cy = cell.row as f32 + 0.5;
            let half = cell.scale * 0.5;
            let gap = 0.08 * cell.scale;
            let x0 = cx - half + gap;
            let x1 = cx + half - gap;
            let y0 = -(cy - half + gap);
            let y1 = -(cy + half - gap);
            let z0 = 0.0;
            let z1 = cube_depth * cell.scale;
            let n_front = [0.0f32, 0.0, 1.0];

            // Just front face + top face for dissolving cells (simpler, faster)
            let base = sv.len() as u32;
            sv.push(Vertex { position: [x0, y0, z1], normal: n_front, color: bright_color });
            sv.push(Vertex { position: [x1, y0, z1], normal: n_front, color: bright_color });
            sv.push(Vertex { position: [x1, y1, z1], normal: n_front, color: bright_color });
            sv.push(Vertex { position: [x0, y1, z1], normal: n_front, color: bright_color });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

            let n_top = [0.0f32, 1.0, 0.0];
            let top_color = [1.0, 1.0, 1.0, alpha * 0.8];
            let base = sv.len() as u32;
            sv.push(Vertex { position: [x0, y0, z1], normal: n_top, color: top_color });
            sv.push(Vertex { position: [x0, y0, z0], normal: n_top, color: top_color });
            sv.push(Vertex { position: [x1, y0, z0], normal: n_top, color: top_color });
            sv.push(Vertex { position: [x1, y0, z1], normal: n_top, color: top_color });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    // HUD
    let (hv, hi) = build_hud(world);

    ((sv, si), (hv, hi))
}

/// Background geometric field: hex grid + connecting web + beat rings
fn build_background(sv: &mut Vec<Vertex>, si: &mut Vec<u32>, world: &GameWorld, gw: f32, gh: f32) {
    let d = world.danger_level;
    let geo_cx = gw / 2.0;
    let geo_cy = -gh / 2.0;
    let geo_z = -2.0;
    let geo_n = [0.0f32, 0.0, 1.0];
    let geo_time = world.preview_angle * (0.3 + d * 0.4);
    let geo_alpha = 0.03 + world.mids * 0.15 + d * 0.05;

    // Hex dot grid
    let hex_rings = 4;
    let dot_size = 0.12 + world.mids * 0.12;
    for ring in 1..=hex_rings {
        let r = ring as f32 * 3.5;
        let points = ring * 6;
        for i in 0..points {
            let angle = (i as f32 / points as f32) * std::f32::consts::TAU + geo_time;
            let dx = angle.cos() * r;
            let dy = angle.sin() * r;
            let dist_factor = 1.0 - (ring as f32 / hex_rings as f32) * 0.5;
            let dot_alpha = geo_alpha * dist_factor;
            let dot_color = [0.15 + d * 0.45, 0.2 - d * 0.08, 0.5 - d * 0.35, dot_alpha];

            let base = sv.len() as u32;
            sv.push(Vertex { position: [geo_cx + dx - dot_size, geo_cy + dy - dot_size, geo_z], normal: geo_n, color: dot_color });
            sv.push(Vertex { position: [geo_cx + dx + dot_size, geo_cy + dy - dot_size, geo_z], normal: geo_n, color: dot_color });
            sv.push(Vertex { position: [geo_cx + dx + dot_size, geo_cy + dy + dot_size, geo_z], normal: geo_n, color: dot_color });
            sv.push(Vertex { position: [geo_cx + dx - dot_size, geo_cy + dy + dot_size, geo_z], normal: geo_n, color: dot_color });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    // Connecting lines
    for ring in 1..=hex_rings {
        let r = ring as f32 * 3.5;
        let points = ring * 6;
        let line_alpha = geo_alpha * 0.4;
        let line_color = [0.1 + d * 0.35, 0.15 - d * 0.05, 0.35 - d * 0.25, line_alpha];
        let line_w = 0.03;
        for i in 0..points {
            let a0 = (i as f32 / points as f32) * std::f32::consts::TAU + geo_time;
            let a1 = ((i + 1) as f32 / points as f32) * std::f32::consts::TAU + geo_time;
            let x0 = geo_cx + a0.cos() * r;
            let y0 = geo_cy + a0.sin() * r;
            let x1 = geo_cx + a1.cos() * r;
            let y1 = geo_cy + a1.sin() * r;
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.001 { continue; }
            let nx = -dy / len * line_w;
            let ny = dx / len * line_w;

            let base = sv.len() as u32;
            sv.push(Vertex { position: [x0 + nx, y0 + ny, geo_z], normal: geo_n, color: line_color });
            sv.push(Vertex { position: [x1 + nx, y1 + ny, geo_z], normal: geo_n, color: line_color });
            sv.push(Vertex { position: [x1 - nx, y1 - ny, geo_z], normal: geo_n, color: line_color });
            sv.push(Vertex { position: [x0 - nx, y0 - ny, geo_z], normal: geo_n, color: line_color });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    // Beat rings
    let ring_cx = gw / 2.0;
    let ring_cy = -gh / 2.0;
    let ring_z = -1.0;
    let ring_n = [0.0f32, 0.0, 1.0];
    let ring_segments = 32;

    for ring in &world.bg_rings {
        let progress = 1.0 - ring.life / ring.max_life;
        let alpha = ring.color[3] * (1.0 - progress).powi(2);
        if alpha < 0.005 { continue; }
        let inner_r = ring.radius;
        let outer_r = ring.radius + 0.3 + (1.0 - progress) * 0.5;
        let color_inner = [ring.color[0], ring.color[1], ring.color[2], alpha];
        let color_outer = [ring.color[0] * 0.5, ring.color[1] * 0.5, ring.color[2] * 0.5, 0.0];

        for i in 0..ring_segments {
            let a0 = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / ring_segments as f32) * std::f32::consts::TAU;
            let (c0, s0) = (a0.cos(), a0.sin());
            let (c1, s1) = (a1.cos(), a1.sin());

            let base = sv.len() as u32;
            sv.push(Vertex { position: [ring_cx + c0 * inner_r, ring_cy + s0 * inner_r, ring_z], normal: ring_n, color: color_inner });
            sv.push(Vertex { position: [ring_cx + c1 * inner_r, ring_cy + s1 * inner_r, ring_z], normal: ring_n, color: color_inner });
            sv.push(Vertex { position: [ring_cx + c1 * outer_r, ring_cy + s1 * outer_r, ring_z], normal: ring_n, color: color_outer });
            sv.push(Vertex { position: [ring_cx + c0 * outer_r, ring_cy + s0 * outer_r, ring_z], normal: ring_n, color: color_outer });
            si.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }
}


/// HUD overlay in screen space
fn build_hud(world: &GameWorld) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut indices = Vec::new();
    let t = &THEME;
    let w = t.win_w as f32;
    let h = t.win_h as f32;
    let text_col = rgba_to_f32(t.text_color);
    let dim_col = rgba_to_f32(t.dim_color);

    // --- Fadeable HUD elements (affected by hud_opacity) ---

    // Next piece panel background (fades with HUD)
    let np_x = w - 120.0;
    let np_y = 12.0;
    push_panel(&mut verts, &mut indices, np_x, np_y, 108.0, 85.0, 0.03);

    // Mark where preview piece starts (won't be faded)
    let preview_start_vert = verts.len();
    let next_type_idx = world.session.bag.peek();
    let next_type = TETROMINO_TYPES[next_type_idx];
    let next_cells = piece_cells(next_type, 0);
    let next_color = rgba_to_f32(piece_color(next_type_idx as u32));
    // Correct preview scale for window aspect ratio
    let theme_aspect = w / h;
    let aspect_corr = theme_aspect / world.window_aspect;
    let preview_scale = 18.0;
    let cube_half = 0.42;
    let preview_cx = np_x + 54.0;
    let preview_cy = np_y + 52.0;

    // 3-axis rotation
    let ax = world.preview_angle * 0.3;
    let ay = world.preview_angle * 0.7;
    let az = world.preview_angle * 0.15;
    let (sx_r, cx_r) = (ax.sin(), ax.cos());
    let (sy, cy) = (ay.sin(), ay.cos());
    let (sz, cz) = (az.sin(), az.cos());

    let mut center = [0.0f32; 3];
    for &(dr, dc) in &next_cells {
        center[0] += dc as f32;
        center[1] += dr as f32;
    }
    center[0] /= next_cells.len() as f32;
    center[1] /= next_cells.len() as f32;

    for &(dr, dc) in &next_cells {
        let local_x = dc as f32 - center[0];
        let local_y = dr as f32 - center[1];
        let corners_local: [[f32; 3]; 8] = [
            [local_x - cube_half, local_y - cube_half, -cube_half],
            [local_x + cube_half, local_y - cube_half, -cube_half],
            [local_x + cube_half, local_y + cube_half, -cube_half],
            [local_x - cube_half, local_y + cube_half, -cube_half],
            [local_x - cube_half, local_y - cube_half, cube_half],
            [local_x + cube_half, local_y - cube_half, cube_half],
            [local_x + cube_half, local_y + cube_half, cube_half],
            [local_x - cube_half, local_y + cube_half, cube_half],
        ];

        let mut projected = [[0.0f32; 2]; 8];
        for (i, c) in corners_local.iter().enumerate() {
            let y1 = c[1] * cx_r - c[2] * sx_r;
            let z1 = c[1] * sx_r + c[2] * cx_r;
            let x2 = c[0] * cy + z1 * sy;
            let z2 = -c[0] * sy + z1 * cy;
            let x3 = x2 * cz - y1 * sz;
            let y3 = x2 * sz + y1 * cz;
            let persp = 4.0 / (4.0 + z2 * 0.3);
            projected[i] = [
                preview_cx + x3 * preview_scale * persp * aspect_corr,
                preview_cy + y3 * preview_scale * persp,
            ];
        }

        let faces: &[([usize; 4], [f32; 3])] = &[
            ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
            ([5, 1, 2, 6], [1.0, 0.0, 0.0]),
            ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
            ([0, 3, 7, 4], [-1.0, 0.0, 0.0]),
            ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
            ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ];

        let mut face_order: Vec<(usize, f32)> = faces.iter().enumerate().map(|(i, (_, n))| {
            let _ny1 = n[1] * cx_r - n[2] * sx_r;
            let nz1 = n[1] * sx_r + n[2] * cx_r;
            let nz2 = -n[0] * sy + nz1 * cy;
            (i, nz2)
        }).collect();
        face_order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        for &(fi, nz) in &face_order {
            if nz < 0.0 { continue; }
            let (ci, _normal) = &faces[fi];
            let shade = 0.4 + nz * 0.6;
            let fc = [next_color[0] * shade, next_color[1] * shade, next_color[2] * shade, next_color[3]];

            let base = verts.len() as u32;
            for &idx in ci {
                let px = projected[idx];
                let (nx, ny) = px_to_ndc(px[0], px[1], w, h);
                verts.push(Vertex { position: [nx, ny, 0.06], normal: HUD_NORMAL, color: fc });
            }
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    let preview_end_vert = verts.len();

    // Score
    push_text(&mut verts, &mut indices, 12.0, 12.0, "SCORE", dim_col, 1.0);
    push_text(&mut verts, &mut indices, 12.0, 24.0, &format!("{}", world.session.score), text_col, 2.0);

    // Level
    push_text(&mut verts, &mut indices, 12.0, 50.0, "LEVEL", dim_col, 1.0);
    let level = level_for_lines(world.session.total_lines);
    push_text(&mut verts, &mut indices, 12.0, 62.0, &format!("{}", level), text_col, 2.0);

    // Lines
    push_text(&mut verts, &mut indices, 12.0, 88.0, "LINES", dim_col, 1.0);
    push_text(&mut verts, &mut indices, 12.0, 100.0, &format!("{}", world.session.total_lines), text_col, 2.0);

    // Now playing
    let track_name = if let Ok(audio) = world.audio.try_lock() {
        audio.track_name.clone()
    } else {
        String::new()
    };
    if !track_name.is_empty() {
        let display_name: String = track_name.chars().take(20).collect();
        let tw = display_name.len() as f32 * 4.0;
        push_text(&mut verts, &mut indices, (w - tw) / 2.0, h - 20.0, &display_name.to_uppercase(), dim_col, 1.0);
    }

    // State overlays
    if world.session.state == GameState::GameOver {
        push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([180, 0, 0, 60]), 0.08);
        let go_w = 200.0; let go_h = 50.0;
        let go_x = (w - go_w) / 2.0;
        let go_y = (h - go_h) / 2.0;
        push_panel(&mut verts, &mut indices, go_x, go_y, go_w, go_h, 0.09);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 8.0, "GAME OVER", rgba_to_f32([255, 80, 80, 255]), 2.0);
        push_text(&mut verts, &mut indices, go_x + 12.0, go_y + 30.0, "ENTER TO RESTART", dim_col, 1.0);
    }

    if world.session.state == GameState::Paused {
        push_quad(&mut verts, &mut indices, 0.0, 0.0, w, h, rgba_to_f32([0, 0, 0, 60]), 0.08);
        let pa_w = 220.0; let pa_h = 120.0;
        let pa_x = (w - pa_w) / 2.0;
        let pa_y = (h - pa_h) / 2.0;
        push_panel(&mut verts, &mut indices, pa_x, pa_y, pa_w, pa_h, 0.09);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 8.0, "PAUSED", rgba_to_f32([255, 255, 100, 255]), 2.0);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 30.0, "L-R  ORBIT", dim_col, 2.0);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 50.0, "L-R MOVE", dim_col, 1.0);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 62.0, "DN  DROP", dim_col, 1.0);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 74.0, "SPC HARD", dim_col, 1.0);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 86.0, "UP CW  Z CCW", dim_col, 1.0);
        push_text(&mut verts, &mut indices, pa_x + 16.0, pa_y + 98.0, "P RESUME", dim_col, 1.0);
    }

    // Particles (always visible, not affected by HUD fade)
    world.particles.render(&mut verts, &mut indices);

    // Apply HUD opacity — skip preview piece and particles
    let opacity = world.hud_opacity;
    if opacity < 0.99 {
        let particle_verts = world.particles.particles.len() * 4;
        let hud_vert_count = verts.len().saturating_sub(particle_verts);
        for (i, v) in verts[..hud_vert_count].iter_mut().enumerate() {
            // Skip preview piece vertices (always visible)
            if i >= preview_start_vert && i < preview_end_vert {
                continue;
            }
            v.color[3] *= opacity;
        }
    }

    (verts, indices)
}
