// Theme presets — parameter bundles for effect modules.

/// Per-effect enabled/disabled flags. Every visual effect is toggleable per-theme.
#[derive(Clone)]
pub struct EffectFlags {
    // Migrated modules
    pub beat_rings: bool,
    pub hex_background: bool,
    pub grid_lines: bool,
    pub fft_visualizer: bool,
    pub fireworks: bool,
    pub fire: bool,
    pub starfield: bool,
    pub aurora: bool,
    pub flow_field: bool,
    pub fluid: bool,
    pub crystal: bool,
    pub camera_sway: bool,

    // Inline scene effects
    pub cube_glow: bool,
    pub ghost_piece: bool,
    pub active_piece_pulse: bool,
    pub clearing_flash: bool,
    pub t_spin_flash: bool,
    pub level_up_rings: bool,
    pub combo_text: bool,

    // Inline world effects
    pub particle_beat_pulse: bool,
    pub line_clear_particles: bool,
    pub camera_shake: bool,
    pub grid_distortion: bool,     // grid line warping on beats/events
}

impl EffectFlags {
    pub fn all_on() -> Self {
        EffectFlags {
            beat_rings: true, hex_background: true, grid_lines: true,
            fft_visualizer: true, fireworks: true, fire: false, starfield: false, aurora: false, flow_field: false, fluid: false, crystal: false, camera_sway: true,
            cube_glow: true, ghost_piece: true, active_piece_pulse: true,
            clearing_flash: true, t_spin_flash: true, level_up_rings: true,
            combo_text: true, particle_beat_pulse: true,
            line_clear_particles: true, camera_shake: true,
            grid_distortion: false,
        }
    }
    pub fn all_off() -> Self {
        EffectFlags {
            beat_rings: false, hex_background: false, grid_lines: false,
            fft_visualizer: false, fireworks: false, fire: false, starfield: false, aurora: false, flow_field: false, fluid: false, crystal: false, camera_sway: false,
            cube_glow: false, ghost_piece: false, active_piece_pulse: false,
            clearing_flash: false, t_spin_flash: false, level_up_rings: false,
            combo_text: false, particle_beat_pulse: false,
            line_clear_particles: false, camera_shake: false,
            grid_distortion: false,
        }
    }
}

/// Which analysis rank drives an effect.
#[derive(Clone, Copy)]
pub enum SignalRank {
    First,          // most active/rhythmic band
    Second,         // second most
    Third,          // third most
    Fixed(usize),   // always use this band index (0-6)
}

/// Maps effects to analysis ranks. The runtime resolves ranks to actual
/// band indices based on rolling_energy + beat_confidence analysis.
#[derive(Clone)]
#[allow(dead_code)]
pub struct EffectBindings {
    pub board_pulse: SignalRank,
    pub cube_glow: SignalRank,
    pub beat_rings: SignalRank,
    pub fireworks: SignalRank,
    pub particles: SignalRank,
    pub camera_sway: SignalRank,
    pub grid_shimmer: SignalRank,
    pub hex_dots: SignalRank,
}

impl EffectBindings {
    pub fn default_bindings() -> Self {
        EffectBindings {
            board_pulse: SignalRank::First,
            cube_glow: SignalRank::Second,
            beat_rings: SignalRank::Fixed(0),   // sub-bass (legacy behavior until wired)
            fireworks: SignalRank::Fixed(1),    // bass (legacy)
            particles: SignalRank::Third,
            camera_sway: SignalRank::Fixed(1),  // bass (legacy)
            grid_shimmer: SignalRank::Fixed(5), // presence (legacy)
            hex_dots: SignalRank::Fixed(2),     // low-mids (legacy)
        }
    }
}

/// Parameters for BeatRings effect.
#[derive(Clone, Copy)]
pub struct RingParams {
    pub max_radius: f32,
    pub base_life: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub base_alpha: f32,
}

/// Parameters for HexBackground effect.
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
pub struct GridParams {
    pub base_r: f32,
    pub base_g: f32,
    pub base_b: f32,
    pub base_thickness: f32,
    pub beat_thickness_add: f32,
}

/// Parameters for FftVisualizer band colors.
#[derive(Clone, Copy)]
pub struct FftParams {
    pub band_colors: [[u8; 3]; 7],
}

/// Parameters for CameraReactor.
#[derive(Clone, Copy)]
#[allow(dead_code)]
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
    pub color_grade: [f32; 3], // post-process color temperature [r, g, b] multiplier
    pub rings: RingParams,
    pub hex: HexParams,
    pub grid: GridParams,
    pub fft: FftParams,
    pub camera: CameraParams,
    pub effects: EffectFlags,
    pub bindings: EffectBindings,
    pub piece_colors: Option<[[u8; 4]; 7]>,
}

pub fn default_theme() -> VisualTheme {
    VisualTheme {
        name: "Default",
        color_grade: [1.05, 1.0, 0.95], // slightly warm
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
            sway_base: 0.15, sway_danger_add: 0.1,
            jitter_x: 0.04, jitter_y: 0.025,
            zoom_amount: 0.7, shake_decay: 1.3,
        },
        effects: {
            let mut f = EffectFlags::all_on();
            f.hex_background = false;
            f
        },
        bindings: EffectBindings::default_bindings(),
        piece_colors: None,
    }
}

pub fn water_theme() -> VisualTheme {
    VisualTheme {
        name: "Water",
        color_grade: [0.92, 0.97, 1.08], // subtle cool blue
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
            sway_base: 0.2, sway_danger_add: 0.08,
            jitter_x: 0.015, jitter_y: 0.01,
            zoom_amount: 0.21, // gentle — less nausea than default
            shake_decay: 0.8,
        },
        effects: {
            let mut f = EffectFlags::all_on();
            f.fireworks = false;
            f
        },
        bindings: EffectBindings::default_bindings(),
        piece_colors: Some([
            [100, 180, 255, 255], // I — light blue
            [ 60, 140, 220, 255], // O — medium blue
            [ 80, 120, 200, 255], // T — blue-purple
            [ 40, 160, 180, 255], // S — teal
            [ 30, 100, 180, 255], // Z — deep blue
            [ 20,  60, 160, 255], // J — navy
            [ 60, 200, 200, 255], // L — cyan
        ]),
    }
}

pub fn space_theme() -> VisualTheme {
    VisualTheme {
        name: "Space",
        color_grade: [0.85, 0.88, 1.15], // cold blue-purple
        rings: RingParams {
            max_radius: 22.0, base_life: 5.0,
            color_r: 0.2, color_g: 0.1, color_b: 0.5, base_alpha: 0.2,
        },
        hex: HexParams {
            dot_min_size: 0.03, dot_max_size: 0.12, base_speed: 0.08,
            danger_speed_mult: 0.15,
            base_r: 0.15, base_g: 0.08, base_b: 0.35, base_alpha: 0.03,
            hex_rings: 5, ring_spacing: 3.2,
        },
        grid: GridParams {
            base_r: 30.0, base_g: 25.0, base_b: 80.0,
            base_thickness: 0.015, beat_thickness_add: 0.025,
        },
        fft: FftParams {
            band_colors: [
                [40, 10, 120], [60, 20, 160], [80, 40, 200],
                [100, 60, 220], [140, 100, 255], [180, 140, 255], [220, 200, 255],
            ],
        },
        camera: CameraParams {
            sway_base: 0.12, sway_danger_add: 0.06,
            jitter_x: 0.02, jitter_y: 0.015,
            zoom_amount: 0.84, shake_decay: 1.0,
        },
        effects: {
            let mut f = EffectFlags::all_on();
            f.fire = false;
            f.hex_background = false;
            f.beat_rings = false;
            f.fireworks = false;
            f.particle_beat_pulse = false;
            f.line_clear_particles = false;
            // starfield + aurora are the stars of this theme
            f.starfield = true;
            f.aurora = true;
            f
        },
        bindings: EffectBindings::default_bindings(),
        piece_colors: Some([
            [140, 120, 255, 255], // I — lavender
            [180, 160, 255, 255], // O — light purple
            [100,  80, 220, 255], // T — deep violet
            [ 80, 200, 255, 255], // S — cyan
            [200, 100, 255, 255], // Z — magenta
            [ 50,  60, 180, 255], // J — dark blue
            [120, 220, 255, 255], // L — ice blue
        ]),
    }
}

// TODO: Crystal theme is WIP — board/piece contrast against white background,
// explosion fragment tuning, fog density balance.
pub fn crystal_theme() -> VisualTheme {
    VisualTheme {
        name: "Crystal",
        color_grade: [1.0, 1.0, 1.0], // neutral — white background needs no grading
        rings: RingParams {
            max_radius: 20.0, base_life: 4.0,
            color_r: 0.1, color_g: 0.1, color_b: 0.1, base_alpha: 0.1,
        },
        hex: HexParams {
            dot_min_size: 0.04, dot_max_size: 0.15, base_speed: 0.1,
            danger_speed_mult: 0.2,
            base_r: 0.05, base_g: 0.05, base_b: 0.05, base_alpha: 0.02,
            hex_rings: 4, ring_spacing: 3.5,
        },
        grid: GridParams {
            base_r: 30.0, base_g: 30.0, base_b: 40.0,
            base_thickness: 0.02, beat_thickness_add: 0.02,
        },
        fft: FftParams {
            band_colors: [
                [20, 20, 30], [30, 30, 45], [40, 40, 60],
                [50, 50, 75], [60, 60, 90], [70, 70, 100], [80, 80, 110],
            ],
        },
        camera: CameraParams {
            sway_base: 0.0, sway_danger_add: 0.0,
            jitter_x: 0.0, jitter_y: 0.0,
            zoom_amount: 0.56, shake_decay: 0.9,
        },
        effects: {
            let mut f = EffectFlags::all_on();
            f.fire = false;
            f.starfield = false;
            f.aurora = false;
            f.hex_background = false;
            f.fireworks = false;
            f.beat_rings = false;
            f.flow_field = false;
            f.fluid = false;
            f.particle_beat_pulse = false;
            f.crystal = true;
            f
        },
        bindings: EffectBindings::default_bindings(),
        piece_colors: Some([
            [ 30,  30,  50, 255], // I — dark slate
            [ 40,  40,  60, 255], // O — charcoal
            [ 20,  20,  40, 255], // T — deep navy
            [ 50,  50,  70, 255], // S — steel
            [ 25,  25,  45, 255], // Z — dark blue
            [ 15,  15,  35, 255], // J — near black
            [ 45,  45,  65, 255], // L — medium slate
        ]),
    }
}

// TODO: Fluid theme is WIP — tumbling piece turbulence, red/white palette.
// Needs: color palette tuning, danger escalation, line clear signature effect.
pub fn fluid_theme() -> VisualTheme {
    VisualTheme {
        name: "Fluid",
        color_grade: [1.05, 0.95, 0.92], // warm slight red shift
        rings: RingParams {
            max_radius: 20.0, base_life: 4.0,
            color_r: 0.4, color_g: 0.15, color_b: 0.1, base_alpha: 0.12,
        },
        hex: HexParams {
            dot_min_size: 0.04, dot_max_size: 0.15, base_speed: 0.1,
            danger_speed_mult: 0.2,
            base_r: 0.3, base_g: 0.1, base_b: 0.1, base_alpha: 0.02,
            hex_rings: 4, ring_spacing: 3.5,
        },
        grid: GridParams {
            base_r: 50.0, base_g: 25.0, base_b: 25.0,
            base_thickness: 0.015, beat_thickness_add: 0.02,
        },
        fft: FftParams {
            band_colors: [
                [100, 20, 20], [140, 30, 30], [180, 50, 40],
                [200, 80, 60], [220, 120, 80], [240, 160, 120], [255, 200, 180],
            ],
        },
        camera: CameraParams {
            sway_base: 0.0, sway_danger_add: 0.0,
            jitter_x: 0.0, jitter_y: 0.0,
            zoom_amount: 0.56, shake_decay: 0.9,
        },
        effects: {
            let mut f = EffectFlags::all_on();
            f.fire = false;
            f.starfield = false;
            f.aurora = false;
            f.hex_background = false;
            f.fireworks = false;
            f.beat_rings = false;
            f.flow_field = false;
            f.particle_beat_pulse = false;
            f.fluid = true;
            f
        },
        bindings: EffectBindings::default_bindings(),
        piece_colors: Some([
            [220, 60, 60, 255],   // I — red
            [255, 255, 245, 255], // O — white
            [200, 50, 50, 255],   // T — deep red
            [255, 240, 230, 255], // S — warm white
            [180, 40, 40, 255],   // Z — dark red
            [255, 220, 210, 255], // J — pink white
            [240, 80, 70, 255],   // L — bright red
        ]),
    }
}

// TODO: Flow theme needs more love — custom color palette tuning, possibly
// flow-field-aware grid lines, particle size/density tied to danger level,
// and a signature visual for line clears (vortex implosion?).
pub fn flow_theme() -> VisualTheme {
    VisualTheme {
        name: "Flow",
        color_grade: [0.95, 1.0, 1.1], // cool neutral with slight blue lift
        rings: RingParams {
            max_radius: 20.0, base_life: 4.0,
            color_r: 0.15, color_g: 0.3, color_b: 0.5, base_alpha: 0.15,
        },
        hex: HexParams {
            dot_min_size: 0.04, dot_max_size: 0.15, base_speed: 0.1,
            danger_speed_mult: 0.2,
            base_r: 0.1, base_g: 0.2, base_b: 0.4, base_alpha: 0.02,
            hex_rings: 4, ring_spacing: 3.5,
        },
        grid: GridParams {
            base_r: 25.0, base_g: 40.0, base_b: 70.0,
            base_thickness: 0.015, beat_thickness_add: 0.02,
        },
        fft: FftParams {
            band_colors: [
                [20, 30, 100], [30, 60, 140], [40, 100, 160],
                [50, 140, 160], [80, 170, 140], [120, 180, 120], [160, 200, 140],
            ],
        },
        camera: CameraParams {
            sway_base: 0.1, sway_danger_add: 0.05,
            jitter_x: 0.015, jitter_y: 0.01,
            zoom_amount: 0.56, shake_decay: 0.9,
        },
        effects: {
            let mut f = EffectFlags::all_on();
            f.fire = false;
            f.starfield = false;
            f.aurora = false;
            f.hex_background = false;
            f.fireworks = false;
            f.beat_rings = false;
            f.particle_beat_pulse = false;
            // Flow field is the star
            f.flow_field = true;
            f
        },
        bindings: EffectBindings::default_bindings(),
        piece_colors: Some([
            [ 80, 180, 200, 255], // I — teal
            [ 60, 160, 160, 255], // O — dark teal
            [100, 140, 200, 255], // T — steel blue
            [ 80, 200, 140, 255], // S — seafoam
            [ 60, 120, 180, 255], // Z — ocean blue
            [ 40,  80, 160, 255], // J — deep blue
            [100, 220, 180, 255], // L — aquamarine
        ]),
    }
}

pub fn debug_theme() -> VisualTheme {
    let mut theme = default_theme();
    theme.name = "Debug";
    theme.effects = EffectFlags::all_off();
    // Baseline experience (always on) + effect under test
    // NOTE: when testing a new effect in isolation, keep these baseline flags on
    // and only add the effect being tested. Toggle other effects off as needed.
    theme.effects.grid_lines = true;
    theme.effects.cube_glow = true;
    theme.effects.ghost_piece = true;
    theme.effects.active_piece_pulse = true;
    theme.effects.clearing_flash = true;
    theme.effects.line_clear_particles = true;
    // Effect under test: (swap this flag to isolate different effects)
    theme.effects.aurora = true;
    theme.color_grade = [1.0, 1.0, 1.0]; // neutral for debug
    // Bindings: rings follow most active band, board pulse follows beat,
    // fireworks follow second most active
    theme.bindings = EffectBindings {
        board_pulse: SignalRank::First,     // most rhythmic band → board depth pulse
        cube_glow: SignalRank::Second,
        beat_rings: SignalRank::First,      // most active band → rings
        fireworks: SignalRank::Second,      // second most active → fireworks
        particles: SignalRank::Third,
        camera_sway: SignalRank::Fixed(1),
        grid_shimmer: SignalRank::Fixed(5),
        hex_dots: SignalRank::Fixed(2),
    };
    theme
}
