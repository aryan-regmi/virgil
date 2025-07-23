use bincode::{Decode, Encode};

/// The status of the Rust response.
#[repr(u8)]
pub enum MessageStatus {
    Success,
    Error,
}

impl TryFrom<u8> for MessageStatus {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, <MessageStatus as TryFrom<u8>>::Error> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::Error),
            _ => Err(format!("Unable to decode message status from u8: {value}")),
        }
    }
}

/// Represents a message sent **from** Rust **to** Dart.
pub trait Message: Encode + Decode<()> {
    /// Gets the length (in bytes) of `self`.
    fn byte_len(&self) -> usize;
}

/// Returns an error to Dart.
#[derive(Debug, Encode, Decode)]
pub struct Error(pub String);
impl Message for Error {
    fn byte_len(&self) -> usize {
        self.0.len() + size_of::<Self>()
    }
}

/// Returns the transcript to Dart.
#[derive(Debug, Encode, Decode)]
pub struct Transcript(pub String);
impl Message for Transcript {
    fn byte_len(&self) -> usize {
        self.0.len() + size_of::<Self>()
    }
}
