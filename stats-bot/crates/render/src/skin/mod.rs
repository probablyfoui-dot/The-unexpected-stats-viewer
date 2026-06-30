// `render/src/skin/mod.rs` — Skin rendering public API.

// ---- Imports ---- //
use std::sync::OnceLock;

mod model;
mod output;
mod render;
mod skin;

pub use model::{Pose, Rotation};
pub use output::{OutputType, RenderOutput};
pub use render::Renderer;
pub use skin::Skin;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkinError {
    #[error("invalid skin dimensions: expected 64x64, got {0}x{1}")]
    InvalidDimensions(u32, u32),
    #[error("failed to decode image: {0}")]
    ImageDecode(#[from] image::ImageError),
    #[error("render error: {0}")]
    Render(String),
}

pub type Result<T> = std::result::Result<T, SkinError>;

// ---- Renderer + OnceLock ---- //
fn global_renderer() -> &'static Result<Renderer> {
    static RENDERER: OnceLock<Result<Renderer>> = OnceLock::new();
    RENDERER.get_or_init(|| {
        tracing::info!("Initializing skin renderer (wgpu)…");
        Renderer::new()
    })
}

// ---- render_skin() ---- //
pub fn render(skin: &Skin, pose: &Pose, output: OutputType) -> Result<RenderOutput> {
    global_renderer()
        .as_ref()
        .map_err(|e| SkinError::Render(e.to_string()))?
        .render(skin, pose, output)
}
