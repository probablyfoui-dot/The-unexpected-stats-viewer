// `render/src/lib.rs` — Crate root.

// ---- Canvas module ---- //
pub mod canvas;
// ---- Cards module ---- //
pub mod cards;
pub mod skin;

pub use canvas::{Canvas, Shape, init as init_canvas};
pub use skin::{OutputType, Pose, RenderOutput, Rotation, Skin, SkinError, render as render_skin};
