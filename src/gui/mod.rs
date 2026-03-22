// Co-authored GUI module — NOT owned by the pipeline.
// All rendering, windowing, and visual logic lives here.
// Pipeline agents: do not modify files in src/gui/.

pub mod theme;
pub mod renderer;
pub mod world;
pub mod app;
pub mod audio_output;
pub mod particles;
pub mod effects;
mod font;
mod drawing;
mod input_bridge;
mod scene;
