// Dashboard UI — 3D music control geometry (volume, transport, folder, FFT visualizer).
// Builds world-space extruded shapes for interactive controls.
// Separated from scene.rs to keep scene building focused on game visuals.

use super::drawing::*;
use super::effects::{AudioEffect, RenderContext};
use super::world::GameWorld;

/// Build 3D dashboard controls into the transparent scene geometry.
pub fn build_dashboard(
    tv: &mut Vec<Vertex>, ti: &mut Vec<u32>,
    world: &GameWorld, gw: f32, gh: f32,
) {
    let hud_a = world.hud_opacity;

    // Volume: [-] ====== [+]
    let vol = if let Ok(audio) = world.audio.try_lock() { audio.volume } else { 0.5 };
    let audio_x = 12.5;
    let vol_minus_x = audio_x;
    let vol_btn_w = 0.5;
    let vol_bar_x = vol_minus_x + vol_btn_w + 0.15;
    let vol_plus_x = audio_x + 2.5;
    let vol_bar_w = vol_plus_x - vol_bar_x - 0.15;
    let vol_y = 15.5;
    let vol_h = 0.2;
    let vol_bg = rgba_to_f32([15, 15, 30, (160.0 * hud_a) as u8]);
    push_slab_3d(tv, ti, vol_bar_x, vol_y + 0.15, vol_bar_w, vol_h, 0.15, vol_bg);
    let vol_fill = rgba_to_f32([60, 100, 180, (220.0 * hud_a) as u8]);
    push_slab_3d(tv, ti, vol_bar_x, vol_y + 0.15, vol_bar_w * vol, vol_h, 0.3, vol_fill);

    // Vol down [-]
    {
        let btn = world.buttons.iter().find(|b| b.id == super::world::ButtonId::VolDown).unwrap();
        let col = if btn.hovered {
            rgba_to_f32([200, 140, 140, (255.0 * hud_a) as u8])
        } else {
            rgba_to_f32([160, 160, 200, (200.0 * hud_a) as u8])
        };
        let cx = btn.world_x + btn.world_w * 0.5;
        let cy = btn.world_y + btn.world_h * 0.5;
        let s = 0.15;
        push_extruded_shape(tv, ti, &[
            [cx - s, cy - s * 0.25], [cx + s, cy - s * 0.25],
            [cx + s, cy + s * 0.25], [cx - s, cy + s * 0.25],
        ], 0.0, 0.25, col);
    }

    // Vol up [+]
    {
        let btn = world.buttons.iter().find(|b| b.id == super::world::ButtonId::VolUp).unwrap();
        let col = if btn.hovered {
            rgba_to_f32([140, 200, 140, (255.0 * hud_a) as u8])
        } else {
            rgba_to_f32([160, 160, 200, (200.0 * hud_a) as u8])
        };
        let cx = btn.world_x + btn.world_w * 0.5;
        let cy = btn.world_y + btn.world_h * 0.5;
        let s = 0.15;
        push_extruded_shape(tv, ti, &[
            [cx - s, cy - s * 0.25], [cx + s, cy - s * 0.25],
            [cx + s, cy + s * 0.25], [cx - s, cy + s * 0.25],
        ], 0.0, 0.25, col);
        push_extruded_shape(tv, ti, &[
            [cx - s * 0.25, cy - s], [cx + s * 0.25, cy - s],
            [cx + s * 0.25, cy + s], [cx - s * 0.25, cy + s],
        ], 0.0, 0.25, col);
    }

    // Transport buttons: [<<] [>||] [>>] [SH]
    let transport_ids = [
        super::world::ButtonId::Back,
        super::world::ButtonId::PlayPause,
        super::world::ButtonId::Skip,
        super::world::ButtonId::Shuffle,
    ];
    let is_paused = if let Ok(audio) = world.audio.try_lock() { audio.paused } else { false };
    for &id in &transport_ids {
        let btn = world.buttons.iter().find(|b| b.id == id).unwrap();
        let base_rgb = match id {
            super::world::ButtonId::PlayPause if is_paused => [120, 180, 120],
            super::world::ButtonId::Shuffle => [120, 120, 200],
            _ => [160, 180, 220],
        };
        let icon_col = if btn.hovered {
            rgba_to_f32([
                (base_rgb[0] as u8).saturating_add(60),
                (base_rgb[1] as u8).saturating_add(60),
                (base_rgb[2] as u8).saturating_add(60),
                (255.0 * hud_a) as u8,
            ])
        } else {
            rgba_to_f32([base_rgb[0] as u8, base_rgb[1] as u8, base_rgb[2] as u8, (200.0 * hud_a) as u8])
        };
        let cx = btn.world_x + btn.world_w * 0.5;
        let cy = btn.world_y + btn.world_h * 0.5;
        let z0 = 0.0;
        let z1 = 0.3;
        let s = 0.2;

        match id {
            super::world::ButtonId::Back => {
                push_extruded_shape(tv, ti, &[
                    [cx + s * 0.8, cy - s], [cx + s * 1.1, cy - s],
                    [cx + s * 1.1, cy + s], [cx + s * 0.8, cy + s],
                ], z0, z1, icon_col);
                push_extruded_shape(tv, ti, &[
                    [cx - s, cy], [cx + s * 0.6, cy - s], [cx + s * 0.6, cy + s],
                ], z0, z1, icon_col);
            }
            super::world::ButtonId::PlayPause => {
                if is_paused {
                    push_extruded_shape(tv, ti, &[
                        [cx - s * 0.6, cy - s], [cx + s, cy], [cx - s * 0.6, cy + s],
                    ], z0, z1, icon_col);
                } else {
                    for offset in [-s * 0.5, s * 0.2] {
                        push_extruded_shape(tv, ti, &[
                            [cx + offset, cy - s], [cx + offset + s * 0.3, cy - s],
                            [cx + offset + s * 0.3, cy + s], [cx + offset, cy + s],
                        ], z0, z1, icon_col);
                    }
                }
            }
            super::world::ButtonId::Skip => {
                push_extruded_shape(tv, ti, &[
                    [cx - s, cy - s], [cx + s * 0.6, cy], [cx - s, cy + s],
                ], z0, z1, icon_col);
                push_extruded_shape(tv, ti, &[
                    [cx + s * 0.8, cy - s], [cx + s * 1.1, cy - s],
                    [cx + s * 1.1, cy + s], [cx + s * 0.8, cy + s],
                ], z0, z1, icon_col);
            }
            super::world::ButtonId::Shuffle => {
                let t = s * 0.12;
                let sh = s * 0.9;
                push_extruded_shape(tv, ti, &[
                    [cx - s, cy + s - t], [cx - s, cy + s + t],
                    [cx + sh, cy - sh + t], [cx + sh, cy - sh - t],
                ], z0, z1, icon_col);
                push_extruded_shape(tv, ti, &[
                    [cx + s, cy - s], [cx + s * 0.5, cy - s], [cx + s, cy - s * 0.5],
                ], z0, z1, icon_col);
                push_extruded_shape(tv, ti, &[
                    [cx - s, cy - s - t], [cx - s, cy - s + t],
                    [cx + sh, cy + sh + t], [cx + sh, cy + sh - t],
                ], z0, z1, icon_col);
                push_extruded_shape(tv, ti, &[
                    [cx + s, cy + s], [cx + s * 0.5, cy + s], [cx + s, cy + s * 0.5],
                ], z0, z1, icon_col);
            }
            _ => {}
        }
    }

    // Folder button — 3D extruded folder icon
    {
        let fld = world.buttons.iter().find(|b| b.id == super::world::ButtonId::Folder).unwrap();
        let col = if fld.hovered {
            rgba_to_f32([140, 160, 220, (255.0 * hud_a) as u8])
        } else {
            rgba_to_f32([100, 120, 180, (200.0 * hud_a) as u8])
        };
        let cx = fld.world_x + fld.world_w * 0.5;
        let cy = fld.world_y + fld.world_h * 0.5;
        let w = 0.385;
        let h = 0.28;
        let tab_w = w * 0.35;
        let tab_h = h * 0.2;
        push_extruded_shape(tv, ti, &[
            [cx - w, cy - h + tab_h], [cx + w, cy - h + tab_h],
            [cx + w, cy + h], [cx - w, cy + h],
        ], 0.0, 0.2, col);
        push_extruded_shape(tv, ti, &[
            [cx - w, cy - h], [cx - w + tab_w, cy - h],
            [cx - w + tab_w, cy - h + tab_h], [cx - w, cy - h + tab_h],
        ], 0.0, 0.2, col);
    }

    // FFT visualizer (with dashboard HUD opacity)
    let ef = &world.effects.flags;
    if ef.fft_visualizer {
        let fft_ctx = RenderContext {
            board_width: gw, board_height: gh,
            win_w: super::theme::THEME.win_w as f32, win_h: super::theme::THEME.win_h as f32,
            window_aspect: world.window_aspect,
            preview_angle: world.preview_angle,
            hud_opacity: hud_a,
        };
        world.effects.fft_vis.render(tv, ti, &fft_ctx);
    }
}
