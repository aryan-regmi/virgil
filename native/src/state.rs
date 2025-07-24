use std::{error::Error, sync::mpsc, thread, time::Duration};

use bincode::{Decode, Encode};
use cpal::{
    InputCallbackInfo,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

pub type VirgilResult<T> = Result<T, Box<dyn Error>>;

const EXPECTED_SAMPLE_RATE: usize = 16_000;

#[derive(Encode, Decode)]
pub struct Context {
    pub model_path: String,
    pub wake_words: Vec<String>,
    pub transcript: String,
}

#[unsafe(no_mangle)]
fn listen_for_duration(seconds: usize) {
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .ok_or_else(|| "Default input device not found".to_string())
        .unwrap();
    let config = input_device.default_input_config().unwrap().config();

    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let input_callback = move |data: &[f32], _: &InputCallbackInfo| {
        tx.send(data.into()).unwrap();
    };
    let input_stream = input_device
        .build_input_stream(&config, input_callback, |err| eprintln!("{err}"), None)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    input_stream.play().map_err(|e| eprintln!("{e}")).unwrap();

    thread::spawn(move || {
        while let Ok(data) = rx.recv() {
            println!("Received {} samples", data.len());
        }
    });
    std::thread::sleep(Duration::from_secs(seconds as u64));
}
