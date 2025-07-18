//! This module contains actors.

mod whisper_actor;

use messages::prelude::Context;
use tokio::spawn;

use crate::actors::whisper_actor::WhisperActor;

/// Creates and spawns the actors in the async system.
pub async fn create_actors() {
    // Create actor contexts.
    let whisper_actor_ctx = Context::new();
    let whisper_actor_addr = whisper_actor_ctx.address();

    // Spawn the actors.
    let whisper_actor = WhisperActor::new(whisper_actor_addr.clone());
    spawn(whisper_actor_ctx.run(whisper_actor));
}
