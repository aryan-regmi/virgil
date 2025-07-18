use async_trait::async_trait;
use messages::{
    actor::Actor,
    prelude::{Address, Context, Notifiable},
};
use rinf::{DartSignal, debug_print};
use tokio::task::JoinSet;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::signals::ModelPath;

/// The actor responsible for all Whisper speech recognition.
pub struct WhisperActor {
    state: Option<WhisperState>,

    /// Owned tasks that are canceled when the actor is dropped.
    _owned_tasks: JoinSet<()>,
}

/// Defines the `WhisperActor` as an actor in the async system.
impl Actor for WhisperActor {}

impl WhisperActor {
    pub fn new(self_addr: Address<Self>) -> Self {
        let mut _owned_tasks = JoinSet::new();
        _owned_tasks.spawn(Self::model_path_listener(self_addr.clone()));
        Self {
            state: None,
            _owned_tasks,
        }
    }
}

#[async_trait]
impl Notifiable<ModelPath> for WhisperActor {
    async fn notify(&mut self, msg: ModelPath, _: &Context<Self>) {
        debug_print!("Model Path: {}", msg.path);

        // Load the context and model
        WhisperContext::new_with_params(&msg.path, WhisperContextParameters::default())
            .map_err(|e| debug_print!("Unable to load model: {e}"))
            .and_then(|ctx| {
                // Intialize model state
                self.state = Some(
                    ctx.create_state()
                        .map_err(|e| debug_print!("Unable to create state: {e}"))
                        .expect("Unable to create state"),
                );
                Ok(())
            })
            .expect("Failed to load model");
    }
}

impl WhisperActor {
    async fn model_path_listener(mut self_addr: Address<Self>) {
        let receiver = ModelPath::get_dart_signal_receiver();
        while let Some(signal_pack) = receiver.recv().await {
            let _ = self_addr
                .notify(signal_pack.message)
                .await
                .map_err(|e| debug_print!("ModelPath Listener Error: {e}"))
                .expect("Failed to notify `ModelPath` signal");
        }
    }
}
