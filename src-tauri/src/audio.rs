use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Sender},
    Arc,
};
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
    alive: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = Arc::clone(&alive);

        thread::spawn(move || {
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to initialize audio output: {}", e);
                    alive_clone.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to create audio sink: {}", e);
                    alive_clone.store(false, Ordering::SeqCst);
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
                                    Err(e) => {
                                        tracing::error!("Error decoding audio file: {}", e)
                                    }
                                }
                            }
                            Err(e) => tracing::error!("Error opening audio file: {}", e),
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
                            Err(e) => {
                                tracing::error!("Error decoding embedded audio: {}", e)
                            }
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

            alive_clone.store(false, Ordering::SeqCst);
        });

        Self { sender, alive }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub fn play_file(&self, path: PathBuf, volume: f32) -> Result<(), String> {
        if !self.is_alive() {
            tracing::error!("Cannot play file: audio thread is not alive");
            return Err("Audio thread is not alive".to_string());
        }
        self.sender
            .send(AudioCommand::Play(path, volume))
            .map_err(|e| {
                tracing::error!("Failed to send Play command to audio thread: {}", e);
                e.to_string()
            })
    }

    pub fn play_embedded(&self, bytes: &'static [u8], volume: f32) -> Result<(), String> {
        if !self.is_alive() {
            tracing::error!("Cannot play embedded audio: audio thread is not alive");
            return Err("Audio thread is not alive".to_string());
        }
        self.sender
            .send(AudioCommand::PlayEmbedded(bytes, volume))
            .map_err(|e| {
                tracing::error!("Failed to send PlayEmbedded command to audio thread: {}", e);
                e.to_string()
            })
    }

    pub fn stop(&self) {
        if !self.is_alive() {
            tracing::warn!("Cannot stop audio: audio thread is not alive");
            return;
        }
        if let Err(e) = self.sender.send(AudioCommand::Stop) {
            tracing::error!("Failed to send Stop command to audio thread: {}", e);
        }
    }

    pub fn pause(&self) {
        if !self.is_alive() {
            tracing::warn!("Cannot pause audio: audio thread is not alive");
            return;
        }
        if let Err(e) = self.sender.send(AudioCommand::Pause) {
            tracing::error!("Failed to send Pause command to audio thread: {}", e);
        }
    }

    pub fn resume(&self) {
        if !self.is_alive() {
            tracing::warn!("Cannot resume audio: audio thread is not alive");
            return;
        }
        if let Err(e) = self.sender.send(AudioCommand::Resume) {
            tracing::error!("Failed to send Resume command to audio thread: {}", e);
        }
    }
}

pub struct AudioState(pub std::sync::Arc<AudioPlayer>);
