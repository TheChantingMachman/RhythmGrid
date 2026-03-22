// Camera reactor — audio-driven camera movement.
// Owns shake/sway/jitter/zoom state, applies to camera position each frame.

use super::effects::AudioFrame;

pub struct CameraReactor {
    pub shake_intensity: f32,
    pub shake_time: f32,
}

impl CameraReactor {
    pub fn new() -> Self {
        CameraReactor {
            shake_intensity: 0.0,
            shake_time: 0.0,
        }
    }

    pub fn trigger_shake(&mut self, intensity: f32) {
        self.shake_intensity = intensity.min(1.0);
    }

    pub fn update(&mut self, audio: &AudioFrame) {
        self.shake_time += audio.dt * 30.0;
        self.shake_intensity = (self.shake_intensity - audio.dt * 1.3).max(0.0);
    }

    /// Apply audio-driven offsets to a base camera position.
    /// Returns the modified eye position.
    pub fn apply(&self, audio: &AudioFrame, preview_angle: f32, base_eye: [f32; 3]) -> [f32; 3] {
        // Bass sway — slow drift on sub-bass/bass beats
        let bass_beat = audio.band_beats[0].max(audio.band_beats[1]);
        let sway_amp = 0.3 + audio.danger * 0.2;
        let sway = bass_beat * sway_amp * (preview_angle * 2.0).sin();

        // High-frequency micro-jitter from presence/brilliance beats
        let hi_beat = audio.band_beats[5].max(audio.band_beats[6]);
        let jitter_x = hi_beat * 0.08 * (preview_angle * 7.0).sin();
        let jitter_y = hi_beat * 0.05 * (preview_angle * 11.0).cos();

        // Impact shake
        let shake_x = self.shake_intensity * (self.shake_time * 1.3).sin() * 0.4;
        let shake_y = self.shake_intensity * (self.shake_time * 1.7).cos() * 0.25;

        // Bass zoom
        let bass_zoom = bass_beat * 0.5;

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
