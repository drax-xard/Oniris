//! Oniris audio core: pack decoding, seamless-bed mixing, randomized events,
//! fades, and the sleep timer. This crate has no Android dependencies.

mod audio;
mod engine;
mod error;
mod limiter;
mod mix;

pub use audio::{decode_audio, DecodedAudio};
pub use engine::{
    Engine, LoadedPack, RenderStats, TransportState, DEFAULT_BLOCK_FRAMES, DEFAULT_MEMORY_LIMIT,
};
pub use error::EngineError;
pub use mix::{MixEntry, MixSpec};

/// Re-exported so platform shells can build mix JSON without a second
/// dependency on the pack crate.
pub use oniris_pack::{Manifest, PackError};
