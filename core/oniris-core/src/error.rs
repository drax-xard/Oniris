use std::path::PathBuf;

use thiserror::Error;

use oniris_pack::PackError;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("pack error: {0}")]
    Pack(#[from] PackError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),

    #[error("unsupported audio format {0:?} (this build decodes WAV and Ogg/Opus)")]
    UnsupportedFormat(String),

    #[error("failed to decode {path}: {message}")]
    Decode { path: PathBuf, message: String },

    #[error("pack {0:?} is not loaded")]
    PackNotLoaded(String),

    #[error("invalid mix: {0}")]
    InvalidMix(String),

    #[error("decoded audio budget exceeded: {used} bytes > {limit} bytes")]
    MemoryBudget { used: usize, limit: usize },

    #[error("invalid audio: {0}")]
    InvalidAudio(String),
}
