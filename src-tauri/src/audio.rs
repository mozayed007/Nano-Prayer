use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;

pub enum AudioCommand {
    Play(PathBuf, f32),
    PlayEmbedded(&'static [u8], f32),
    Pause,
    Resume,
    Stop,
}

pub struct AudioPlayer {
    sender: Sender<AudioCommand>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (sender, receiver) = channel();

        thread::spawn(move || {
            // OutputStream must be kept alive on this thread
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to initialize audio output: {}", e);
                    return; // Exit audio thread, commands will be ignored
                }
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create audio sink: {}", e);
                    return;
                }
            };

            while let Ok(command) = receiver.recv() {
                match command {
                    AudioCommand::Play(path, volume) => {
                        if !sink.empty() {
                            sink.stop();
                        }

                        sink.set_volume(volume);

                        match File::open(&path) {
                            Ok(file) => {
                                let reader = BufReader::new(file);
                                match Decoder::new(reader) {
                                    Ok(source) => sink.append(source),
                                    Err(e) => eprintln!("Error decoding audio file: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Error opening audio file: {}", e),
                        }

                        sink.play();
                    }
                    AudioCommand::PlayEmbedded(bytes, volume) => {
                        if !sink.empty() {
                            sink.stop();
                        }

                        sink.set_volume(volume);

                        let cursor = std::io::Cursor::new(bytes);
                        match Decoder::new(cursor) {
                            Ok(source) => sink.append(source),
                            Err(e) => eprintln!("Error decoding embedded audio: {}", e),
                        }

                        sink.play();
                    }
                    AudioCommand::Pause => {
                        sink.pause();
                    }
                    AudioCommand::Resume => {
                        sink.play();
                    }
                    AudioCommand::Stop => {
                        sink.stop();
                    }
                }
            }
        });

        Self { sender }
    }

    pub fn play_file(&self, path: PathBuf, volume: f32) -> Result<(), String> {
        self.sender
            .send(AudioCommand::Play(path, volume))
            .map_err(|e| e.to_string())
    }

    pub fn play_embedded(&self, bytes: &'static [u8], volume: f32) -> Result<(), String> {
        self.sender
            .send(AudioCommand::PlayEmbedded(bytes, volume))
            .map_err(|e| e.to_string())
    }

    pub fn stop(&self) {
        let _ = self.sender.send(AudioCommand::Stop);
    }

    pub fn pause(&self) {
        let _ = self.sender.send(AudioCommand::Pause);
    }

    pub fn resume(&self) {
        let _ = self.sender.send(AudioCommand::Resume);
    }
}

pub struct AudioState(pub std::sync::Arc<AudioPlayer>);
