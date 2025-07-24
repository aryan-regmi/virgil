use bincode::{Decode, Encode};

use crate::utils::Context;

/// Represents a message sent **from** Rust **to** Dart.
pub trait Message: Encode + Decode<()> {
    fn byte_len(&self) -> usize;
}

impl Message for String {
    fn byte_len(&self) -> usize {
        size_of::<Self>() + self.len()
    }
}

impl Message for Vec<String> {
    fn byte_len(&self) -> usize {
        let len = self.iter().fold(
            size_of::<Self>() + self.len() * size_of::<String>(),
            |acc, v| acc + v.byte_len(),
        );
        len
    }
}

impl Message for Context {
    fn byte_len(&self) -> usize {
        size_of::<Self>()
            + self.model_path.byte_len()
            + self.wake_words.byte_len()
            + self.transcript.byte_len()
    }
}
