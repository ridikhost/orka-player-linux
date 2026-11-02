use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct SampleTapper<S> {
    inner: S,
    buffer: Arc<Mutex<Vec<f32>>>,
    counter: usize,
}

impl<S: Source<Item = i16>> Iterator for SampleTapper<S> {
    type Item = i16;
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        self.counter += 1;
        if self.counter % 64 == 0 {
            let f_val = sample as f32 / 32768.0;
            if let Ok(mut buf) = self.buffer.lock() {
                if buf.len() > 150 { buf.remove(0); }
                buf.push(f_val);
            }
        }
        Some(sample)
    }
}

impl<S: Source<Item = i16>> Source for SampleTapper<S> {
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<std::time::Duration> { self.inner.total_duration() }
}

pub struct MediaPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Option<Sink>,
    pub is_playing: bool,
    pub volume: f32,
    pub current_track: Option<String>,
    pub current_path: Option<String>,
    pub duration: f32,
    pub waveform_data: Arc<Mutex<Vec<f32>>>,
    playback_start: Option<Instant>,
    paused_duration: std::time::Duration,
    mpv_process: Option<Child>,
}

impl MediaPlayer {
    pub fn new() -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        Self {
            _stream, stream_handle, sink: None,
            is_playing: false, volume: 0.8,
            current_track: None, current_path: None, duration: 180.0,
            waveform_data: Arc::new(Mutex::new(vec![0.0; 150])),
            playback_start: None,
            paused_duration: std::time::Duration::ZERO,
            mpv_process: None,
        }
    }

    pub fn load_and_play(&mut self, path: &str) {
        // Zatrzymujemy poprzednie odtwarzanie (oraz proces mpv, jeśli działał)
        self.stop();

        // Jeśli to adres sieciowy lub plik .m3u8, uruchamiamy zewnętrzny odtwarzacz MPV w tle
        if path.starts_with("http://") || path.starts_with("https://") || path.ends_with(".m3u8") {
            println!("[MediaPlayer] Uruchamianie strumienia przez MPV: {}", path);
            match Command::new("mpv").arg(path).spawn() {
                Ok(child) => {
                    self.mpv_process = Some(child);
                    self.is_playing = true;
                    self.current_track = Some(path.to_string());
                    self.current_path = Some(path.to_string());
                    self.playback_start = Some(Instant::now());
                    self.paused_duration = std::time::Duration::ZERO;
                    return;
                }
                Err(e) => {
                    eprintln!("[MediaPlayer] Błąd uruchamiania MPV: {}. Upewnij się, że mpv jest zainstalowany w systemie.", e);
                }
            }
        }

        // Standardowe odtwarzanie lokalnych plików audio przez Rodio
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            if let Ok(decoder) = Decoder::new(reader) {
                let tapper = SampleTapper {
                    inner: decoder,
                    buffer: Arc::clone(&self.waveform_data),
                    counter: 0,
                };
                let sink = Sink::try_new(&self.stream_handle).unwrap();
                sink.set_volume(self.volume);
                sink.append(tapper);
                sink.play();

                let path_obj = Path::new(path);
                let file_name = path_obj.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string();

                self.current_track = Some(file_name);
                self.current_path = Some(path.to_string());
                self.sink = Some(sink);
                self.is_playing = true;
                self.playback_start = Some(Instant::now());
                self.paused_duration = std::time::Duration::ZERO;
            }
        }
    }

    pub fn get_current_position(&self) -> f32 {
        if let Some(start) = self.playback_start {
            if self.is_playing {
                start.elapsed().as_secs_f32()
            } else {
                self.paused_duration.as_secs_f32()
            }
        } else {
            0.0
        }
    }

    pub fn toggle_play_pause(&mut self) {
        if self.mpv_process.is_some() {
            // Dla strumieni mpv zatrzymanie/wznowienie realizujemy przez stop/start lub sygnał, 
            // najprościej w IPTV na żywo stop oznacza zamknięcie procesu.
            return;
        }

        if let Some(sink) = &self.sink {
            if self.is_playing {
                sink.pause();
                self.is_playing = false;
                if let Some(start) = self.playback_start {
                    self.paused_duration = start.elapsed();
                }
            } else {
                sink.play();
                self.is_playing = true;
                self.playback_start = Some(Instant::now() - self.paused_duration);
            }
        }
    }

    pub fn stop(&mut self) {
        // Ubijamy proces MPV, jeśli działa
        if let Some(mut child) = self.mpv_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(sink) = &self.sink { sink.stop(); }
        self.sink = None; 
        self.is_playing = false; 
        self.current_track = None; 
        self.current_path = None;
        self.playback_start = None;
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink { sink.set_volume(self.volume); }
    }

    pub fn save_word_sync_lrc(&self, audio_path: &str, word_timestamps: &[(f32, String)]) {
        let path_obj = Path::new(audio_path);
        if let Some(parent) = path_obj.parent() {
            let stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or("song");
            let lrc_path = parent.join(format!("{}.lrc", stem));
            
            if let Ok(mut file) = File::create(&lrc_path) {
                for (time, word) in word_timestamps {
                    let total_secs = *time;
                    let mins = (total_secs / 60.0).floor() as u32;
                    let secs = (total_secs % 60.0).floor() as u32;
                    let millis = ((total_secs % 1.0) * 100.0) as u32;
                    let _ = writeln!(file, "[{:02}:{:02}.{:02}] {}", mins, secs, millis, word);
                }
            }
        }
    }
}
