// Camera reactor — audio-driven camera movement.
// Owns shake/sway/jitter/zoom state, applies to camera position each frame.

use super::effects::AudioFrame;
use super::effects::themes::CameraParams;

pub struct CameraReactor {
    pub shake_intensity: f32,
    pub shake_time: f32,
    smooth_zoom: f32, // smoothed bass zoom (lerps toward target)
    params: CameraParams,
}

impl CameraReactor {
    pub fn new(params: CameraParams) -> Self {
        CameraReactor {
            shake_intensity: 0.0,
            shake_time: 0.0,
            smooth_zoom: 0.0,
            params,
        }
    }

    pub fn trigger_shake(&mut self, intensity: f32) {
        self.shake_intensity = intensity.min(1.0);
    }

    pub fn update(&mut self, audio: &AudioFrame) {
        self.shake_time += audio.dt * 30.0;
        self.shake_intensity = (self.shake_intensity - audio.dt * self.params.shake_decay).max(0.0);
        // Smooth zoom — lerp toward target, faster in than out
        let bass_beat = audio.band_beats[0].max(audio.band_beats[1]);
        let target = bass_beat * self.params.zoom_amount;
        if target > self.smooth_zoom {
            self.smooth_zoom += (target - self.smooth_zoom) * (audio.dt * 8.0).min(1.0); // ease in
        } else {
            self.smooth_zoom += (target - self.smooth_zoom) * (audio.dt * 1.5).min(1.0); // slow ease out
        }
    }

    pub fn apply(&self, audio: &AudioFrame, preview_angle: f32, base_eye: [f32; 3]) -> [f32; 3] {
        let p = &self.params;
        let bass_beat = audio.band_beats[0].max(audio.band_beats[1]);
        let sway_amp = p.sway_base + audio.danger * p.sway_danger_add;
        let sway = bass_beat * sway_amp * (preview_angle * 2.0).sin();

        let hi_beat = audio.band_beats[5].max(audio.band_beats[6]);
        let jitter_x = hi_beat * p.jitter_x * (preview_angle * 7.0).sin();
        let jitter_y = hi_beat * p.jitter_y * (preview_angle * 11.0).cos();

        let shake_x = self.shake_intensity * (self.shake_time * 1.3).sin() * 0.2;
        let shake_y = self.shake_intensity * (self.shake_time * 1.7).cos() * 0.12;

        let bass_zoom = self.smooth_zoom;

        [
            base_eye[0] + sway + jitter_x + shake_x,
            base_eye[1] + jitter_y + shake_y,
            base_eye[2] - bass_zoom,
        ]
    }

    pub fn reset(&mut self) {
        self.shake_intensity = 0.0;
        self.shake_time = 0.0;
        self.smooth_zoom = 0.0;
    }
}
