use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use samplerate::{ConverterType, Samplerate};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

#[derive(Parser)]
struct Args {
    /// Duration to listen to the microphone (in seconds)
    #[arg(short, long, default_value = "5")]
    duration: u64,

    /// Path to Whisper model (e.g. ggml-small.en.bin)
    #[arg(short, long)]
    model: String,
}

fn main() {
    let args = Args::parse();
    let duration_secs = args.duration;
    let whisper_model_path = &args.model;

    // Setup microphone
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("Failed to get default input device");
    let config = device.default_input_config().unwrap();
    let sample_rate = config.sample_rate().0;
    let channels = config.channels as usize;

    println!("Using input device: {}", device.name().unwrap());
    println!(
        "Original sample rate: {} Hz, channels: {}",
        sample_rate, channels
    );
    println!("Listening for {} seconds...", duration_secs);

    // Create channel to send raw audio samples
    let (tx, rx) = mpsc::channel::<Vec<f32>>();

    // Build input stream
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), tx),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config.into(), tx),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config.into(), tx),
    }
    .unwrap();

    stream.play().unwrap();

    // Start recording
    let start_time = Instant::now();
    let mut collected = Vec::<f32>::new();

    while start_time.elapsed().as_secs() < duration_secs {
        if let Ok(samples) = rx.recv_timeout(Duration::from_millis(100)) {
            collected.extend(samples);
        }
    }

    println!("Recording finished. Processing...");

    // Convert stereo to mono
    let mono = stereo_to_mono(&collected, channels);

    // Resample to 16kHz
    let resampled = resample_to_16k(&mono, sample_rate as usize);

    // Transcribe using whisper-rs
    transcribe(&resampled, whisper_model_path);
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sender: Sender<Vec<f32>>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::Sample,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let samples: Vec<f32> = data.iter().map(|s| s.to_f32()).collect();
            if sender.send(samples).is_err() {
                eprintln!("Audio channel closed");
            }
        },
        move |err| {
            eprintln!("Stream error: {:?}", err);
        },
        None,
    )
}

fn stereo_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}

fn resample_to_16k(input: &[f32], original_rate: usize) -> Vec<f32> {
    if original_rate == 16_000 {
        return input.to_vec();
    }

    let ratio = 16_000.0 / original_rate as f32;
    let mut converter = Samplerate::new(ConverterType::SincBestQuality, 1).unwrap();
    converter.process(ratio, &[input.to_vec()]).unwrap()
}

fn transcribe(samples: &[f32], model_path: &str) {
    let ctx = WhisperContext::new(model_path).expect("Failed to load Whisper model");

    let mut state = ctx.create_state().expect("Failed to create state");
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(true);

    println!("Running Whisper...");
    if state.full(params, samples).is_ok() {
        for i in 0..state.full_n_segments() {
            let text = state.full_get_segment_text(i).unwrap();
            println!("[{}] {}", i, text);
        }
    } else {
        eprintln!("Transcription failed");
    }
}
