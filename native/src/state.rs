use std::{
    error::Error,
    sync::mpsc::{self, Sender},
};

use bincode::{Decode, Encode};
use cpal::{
    Device, InputCallbackInfo, SampleRate, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub type VirgilResult<T> = Result<T, Box<dyn Error>>;

const EXPECTED_SAMPLE_RATE: usize = 16_000;

#[derive(Encode, Decode)]
pub struct Context {
    pub model_path: String,
    pub wake_words: Vec<String>,
    pub transcript: String,
}

pub struct Virgil {
    microphone: Device,
    config: SupportedStreamConfig,
    model_path: String,
    wake_words: Vec<String>,
}

impl Virgil {
    pub fn new(model_path: &str, wake_words: Vec<String>) -> VirgilResult<Self> {
        // Setup microphone
        let host = cpal::default_host();
        let microphone = host
            .default_input_device()
            .ok_or_else(|| String::from("Default input device not found"))?;
        let mut supported_configs = microphone.supported_input_configs()?;
        let config = supported_configs
            .next()
            .ok_or_else(|| String::from("No supported configs found for the microphone"))?
            .with_sample_rate(SampleRate(EXPECTED_SAMPLE_RATE as u32));

        Ok(Self {
            microphone,
            config,
            model_path: model_path.into(),
            wake_words,
        })
    }

    /// Listens to and saves the audio data from the microphone.
    pub fn listen(&mut self, transcript: &mut String) -> VirgilResult<()> {
        // Setup Whisper model
        let ctx =
            WhisperContext::new_with_params(&self.model_path, WhisperContextParameters::default())
                .map_err(|e| e.to_string())?;
        let mut model = ctx.create_state().map_err(|e| e.to_string())?;
        let wake_words = self.wake_words.clone();

        // Setup channel for communicating with input stream
        let (sender, receiver) = mpsc::channel::<String>();

        // Setup input stream
        let config = self.config.config();
        let channels = config.channels as usize;
        let error_callback = |err| eprintln!("{err}"); // FIXME: Write to logfile instead!
        let stream = self.microphone.build_input_stream(
            &config,
            move |data: &[f32], _: &InputCallbackInfo| {
                Self::process_audio_data(
                    sender.clone(),
                    &mut model,
                    wake_words.clone(),
                    data,
                    channels,
                )
                .map_err(|e| eprintln!("{e}",)) // FIXME: Write to logfile instead!
                .unwrap();
            },
            error_callback,
            None,
        )?;
        stream.play()?;

        // Update the transcript
        for text in receiver {
            transcript.push_str(&text);
        }

        Ok(())
    }

    // FIXME: Accumulate audio data until it is equal to sample rate!
    //
    /// Process the audio data from the microphone input.
    fn process_audio_data(
        sender: Sender<String>,
        model: &mut WhisperState,
        wake_words: Vec<String>,
        data: &[f32],
        channels: usize,
    ) -> VirgilResult<()> {
        // TODO: Handle stereo (2 channels)
        if let Some(audio_data) = data.chunks(channels).next() {
            let wake_word_detected = Self::detect_wake_words(model, &wake_words, audio_data)?;
            if wake_word_detected {
                let transcript = Self::transcribe(model, audio_data)?;
                sender.send(transcript)?;
            }
        }

        Ok(())
    }

    /// Chekcs if any wake words are present in the audio data.
    fn detect_wake_words(
        model: &mut WhisperState,
        wake_words: &Vec<String>,
        audio_data: &[f32],
    ) -> VirgilResult<bool> {
        if !audio_data.is_empty() {
            let transcript = Self::transcribe(model, audio_data)?
                .to_lowercase()
                .to_lowercase();
            for word in wake_words {
                let word = word.to_lowercase();
                if transcript.contains(&word) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Runs the `Whisper` model and transcribes the provided audio data.
    fn transcribe(model: &mut WhisperState, audio_data: &[f32]) -> VirgilResult<String> {
        // Run model
        model.full(
            FullParams::new(SamplingStrategy::Greedy { best_of: 1 }),
            audio_data,
        )?;

        // Return results
        let mut transcript = String::with_capacity(1026);
        let num_segments = model.full_n_segments().unwrap();
        for i in 0..num_segments {
            let segment = model.full_get_segment_text(i).unwrap();
            transcript.push_str(&segment);
        }

        Ok(transcript.trim().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listening() -> VirgilResult<()> {
        let mut transcript = String::with_capacity(1024);

        let mut virgil = Virgil::new(
            "test_assets/ggml-tiny.bin",
            vec!["Hi", "Test", "Hello", "Wake"]
                .iter()
                .map(|v| (*v).into())
                .collect(),
        )?;

        virgil.listen(&mut transcript)?;

        dbg!(transcript);

        Ok(())
    }
}
