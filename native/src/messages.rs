use bincode::{Decode, Encode};

/// The status of the Rust response.
#[repr(u8)]
#[derive(Debug, Encode, Decode)]
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

// FIXME: Remove [RustMessage], and replace with direct types:
//  - Impl `Message` trait that returns the byte_len of each type
//
/// Represents a message sent **from** Rust **to** Dart.
#[derive(Debug, Encode, Decode)]
pub struct RustMessage {
    pub status: MessageStatus,
    pub byte_len: usize,
    pub message: Vec<u8>,
}
