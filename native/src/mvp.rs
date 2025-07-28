use core::ffi;
use std::time::Duration;

use cpal::traits::StreamTrait;
use tokio::{runtime::Runtime, sync::mpsc};
use tracing::{Level, debug, error, info, span};
use whisper_rs::{FullParams, SamplingStrategy, WhisperState};

use crate::utils::{
    Context, EXPECTED_SAMPLE_RATE, SendStream, VirgilResult, deserialize, detect_wake_words,
    init_microphone, init_model, transcribe,
};

#[unsafe(no_mangle)]
pub fn listen(
    ctx: *mut ffi::c_void,
    ctx_len: usize,
    listen_duration_ms: usize,
    // ctx_out: *mut ffi::c_void,
    // ctx_len_out: *mut usize,
) {
    let span = span!(Level::TRACE, "listen");
    let _enter = span.enter();

    let listen_duration_ms = listen_duration_ms as u64;

    // Init tokio runtime
    let rt = Runtime::new().map_err(|e| error!("{e}")).unwrap();
    let _rt_guard = rt.enter();

    // Setup channels for communication
    let (input_audio_tx, mut input_audio_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);

    // Decode context
    let ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    debug!("Context decoded");

    // Init `Whisper` model
    let mut model = init_model(&ctx.model_path)
        .map_err(|e| error!("{e}"))
        .unwrap();

    // Initalize microphone
    let mic = SendStream(
        init_microphone(input_audio_tx.clone())
            .map_err(|e| error!("{e}"))
            .unwrap(),
    );

    // Listen to the microphone for the specified amount of time
    rt.spawn(async move {
        let span = span!(Level::TRACE, "listener");
        let _enter = span.enter();

        mic.0
            .play()
            .map_err(|e| error!("Failed to start listening to mic: {e}"))
            .unwrap();
        debug!("Listening to microphone...");

        loop {
            tokio::time::sleep(Duration::from_millis(listen_duration_ms)).await;
        }
    });

    let desired_num_samples = (listen_duration_ms as usize / 1000) * EXPECTED_SAMPLE_RATE + 200;
    let mut accumulated_audio = Vec::with_capacity(desired_num_samples);
    loop {
        while let Ok(audio_data) = input_audio_rx.try_recv() {
            let accumulated_samples = accumulated_audio.len();
            let samples_to_add = audio_data.len();
            let num_samples = accumulated_samples + samples_to_add;

            // Accumulate audio data until desired length is reached
            if num_samples < desired_num_samples {
                accumulated_audio.extend_from_slice(&audio_data);
                continue;
            }

            // If more than desired samples, send exact amount then restart accumulation
            if num_samples >= desired_num_samples {
                let extra = num_samples - desired_num_samples;
                let end_idx = samples_to_add - extra;

                // Send desired number of samples
                accumulated_audio.extend_from_slice(&audio_data[0..end_idx]);
                debug!("Accumulated {} samples", accumulated_audio.len());

                // Process data
                process_audio_data(&mut model, &accumulated_audio, &ctx.wake_words)
                    .map_err(|e| error!("Unable to process audio: {e}"))
                    .unwrap();

                // Reset accumulated data and fill with remaining/overflowing samples
                debug!("Accumulated data reset");
                accumulated_audio.clear();
                accumulated_audio.extend_from_slice(&audio_data[end_idx..]);

                continue;
            }
        }

        std::thread::sleep(Duration::from_millis(listen_duration_ms));
    }
}

fn process_audio_data(
    model: &mut WhisperState,
    audio_data: &[f32],
    wake_words: &Vec<String>,
) -> VirgilResult<()> {
    let span = span!(Level::TRACE, "process_audio_data");
    let _enter = span.enter();

    debug!("Processing {} samples", audio_data.len());

    let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let wake_word_detected = detect_wake_words(model, params.clone(), audio_data, wake_words)?;
    if wake_word_detected {
        info!("Wake word detected");
    }

    // TODO: Move into wake_word_detected check
    let text = transcribe(model, params, audio_data)?;
    if !text.is_empty() {
        info!("Text: {text}");
    }

    Ok(())
}
