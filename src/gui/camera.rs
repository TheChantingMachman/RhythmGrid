// Camera reactor — audio-driven camera movement.
// Owns shake/sway/jitter/zoom state, applies to camera position each frame.

use super::effects::AudioFrame;
use super::effects::themes::CameraParams;

pub struct CameraReactor {
    pub shake_intensity: f32,
    pub shake_time: f32,
    params: CameraParams,
}

impl CameraReactor {
    pub fn new(params: CameraParams) -> Self {
        CameraReactor {
            shake_intensity: 0.0,
            shake_time: 0.0,
            params,
        }
    }

    pub fn trigger_shake(&mut self, intensity: f32) {
        self.shake_intensity = intensity.min(1.0);
    }

    pub fn update(&mut self, audio: &AudioFrame) {
        self.shake_time += audio.dt * 30.0;
        self.shake_intensity = (self.shake_intensity - audio.dt * self.params.shake_decay).max(0.0);
    }

    pub fn apply(&self, audio: &AudioFrame, preview_angle: f32, base_eye: [f32; 3]) -> [f32; 3] {
        let p = &self.params;
        let bass_beat = audio.band_beats[0].max(audio.band_beats[1]);
        let sway_amp = p.sway_base + audio.danger * p.sway_danger_add;
        let sway = bass_beat * sway_amp * (preview_angle * 2.0).sin();

        let hi_beat = audio.band_beats[5].max(audio.band_beats[6]);
        let jitter_x = hi_beat * p.jitter_x * (preview_angle * 7.0).sin();
        let jitter_y = hi_beat * p.jitter_y * (preview_angle * 11.0).cos();

        let shake_x = self.shake_intensity * (self.shake_time * 1.3).sin() * 0.4;
        let shake_y = self.shake_intensity * (self.shake_time * 1.7).cos() * 0.25;

        let bass_zoom = bass_beat * p.zoom_amount;

        [
            base_eye[0] + sway + jitter_x + shake_x,
            base_eye[1] + jitter_y + shake_y,
            base_eye[2] - bass_zoom,
        ]
    }

    pub fn reset(&mut self) {
        self.shake_intensity = 0.0;
        self.shake_time = 0.0;
    }
}
