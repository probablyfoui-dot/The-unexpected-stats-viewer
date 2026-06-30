// `render/src/skin/model/mod.rs` — Model module re-exports.

mod parts;
mod pose;
mod uv;

pub use parts::BodyPart;
pub use pose::{Pose, Rotation};
pub(crate) use uv::{CubeFaceUvs, FaceUv, box_uv};
