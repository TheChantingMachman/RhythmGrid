// Theme presets — parameter bundles for effect modules.

/// Parameters for BeatRings effect.
pub struct RingParams {
    pub max_radius: f32,
    pub base_life: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub base_alpha: f32,
}

/// Parameters for HexBackground effect.
pub struct HexParams {
    pub dot_min_size: f32,
    pub dot_max_size: f32,
    pub base_speed: f32,
    pub danger_speed_mult: f32,
    pub base_r: f32,
    pub base_g: f32,
    pub base_b: f32,
    pub base_alpha: f32,
    pub hex_rings: usize,
    pub ring_spacing: f32,
}

/// Parameters for GridLines effect.
pub struct GridParams {
    pub base_r: f32,
    pub base_g: f32,
    pub base_b: f32,
    pub base_thickness: f32,
    pub beat_thickness_add: f32,
}

/// Parameters for FftVisualizer band colors.
pub struct FftParams {
    pub band_colors: [[u8; 3]; 7],
}

/// Parameters for CameraReactor.
pub struct CameraParams {
    pub sway_base: f32,
    pub sway_danger_add: f32,
    pub jitter_x: f32,
    pub jitter_y: f32,
    pub zoom_amount: f32,
    pub shake_decay: f32,
}

/// A complete visual theme — parameters for all effects.
pub struct VisualTheme {
    pub name: &'static str,
    pub rings: RingParams,
    pub hex: HexParams,
    pub grid: GridParams,
    pub fft: FftParams,
    pub camera: CameraParams,
    pub hex_enabled: bool,
    pub fireworks_enabled: bool,
}

pub fn default_theme() -> VisualTheme {
    VisualTheme {
        name: "Default",
        rings: RingParams {
            max_radius: 18.0, base_life: 3.0,
            color_r: 0.1, color_g: 0.15, color_b: 0.4, base_alpha: 0.3,
        },
        hex: HexParams {
            dot_min_size: 0.06, dot_max_size: 0.30, base_speed: 0.3,
            danger_speed_mult: 0.4,
            base_r: 0.15, base_g: 0.2, base_b: 0.5, base_alpha: 0.03,
            hex_rings: 4, ring_spacing: 3.5,
        },
        grid: GridParams {
            base_r: 40.0, base_g: 45.0, base_b: 70.0,
            base_thickness: 0.02, beat_thickness_add: 0.03,
        },
        fft: FftParams {
            band_colors: [
                [30, 30, 180], [40, 80, 180], [40, 160, 160],
                [60, 170, 80], [180, 180, 40], [200, 100, 40], [200, 50, 50],
            ],
        },
        camera: CameraParams {
            sway_base: 0.3, sway_danger_add: 0.2,
            jitter_x: 0.08, jitter_y: 0.05,
            zoom_amount: 0.5, shake_decay: 1.3,
        },
        hex_enabled: true,
        fireworks_enabled: false,
    }
}

pub fn water_theme() -> VisualTheme {
    VisualTheme {
        name: "Water",
        rings: RingParams {
            max_radius: 24.0, base_life: 4.5, // slower, wider ripples
            color_r: 0.05, color_g: 0.25, color_b: 0.5, base_alpha: 0.2,
        },
        hex: HexParams {
            dot_min_size: 0.04, dot_max_size: 0.18, base_speed: 0.15, // slower drift
            danger_speed_mult: 0.2,
            base_r: 0.05, base_g: 0.2, base_b: 0.45, base_alpha: 0.04, // blue-green
            hex_rings: 5, ring_spacing: 3.0, // more, tighter — like bubbles
        },
        grid: GridParams {
            base_r: 20.0, base_g: 50.0, base_b: 80.0, // blue-green grid
            base_thickness: 0.015, beat_thickness_add: 0.02, // subtler pulse
        },
        fft: FftParams {
            band_colors: [ // all blue-cyan-white gradient
                [10, 20, 120], [20, 40, 160], [30, 80, 180],
                [40, 140, 200], [60, 180, 220], [100, 210, 240], [180, 240, 255],
            ],
        },
        camera: CameraParams {
            sway_base: 0.4, sway_danger_add: 0.15,
            jitter_x: 0.03, jitter_y: 0.02,
            zoom_amount: 0.1, // gentle — less nausea than default
            shake_decay: 0.8,
        },
        hex_enabled: false,
        fireworks_enabled: true,
    }
}
