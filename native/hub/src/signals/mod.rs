use rinf::DartSignal;
use serde::Deserialize;

/// The path of the Whisper model.
#[derive(Deserialize, DartSignal)]
pub struct ModelPath {
    pub path: String,
}
