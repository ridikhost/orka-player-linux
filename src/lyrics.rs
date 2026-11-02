use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub timestamp: f32,
    pub text: String,
}

pub fn load_lrc(audio_path: &str) -> Vec<LyricLine> {
    let mut lyrics = Vec::new();
    let path_obj = Path::new(audio_path);
    
    if let Some(parent) = path_obj.parent() {
        let stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let lrc_path = parent.join(format!("{}.lrc", stem));

        if let Ok(file) = File::open(lrc_path) {
            let reader = BufReader::new(file);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    if line.starts_with('[') && line.contains(']') {
                        let parts: Vec<&str> = line.splitn(2, ']').collect();
                        if parts.len() == 2 {
                            let time_str = &parts[0][1..];
                            let text = parts[1].trim().to_string();
                            if let Some(secs) = parse_time_tag(time_str) {
                                lyrics.push(LyricLine { timestamp: secs, text });
                            }
                        }
                    }
                }
            }
        }
    }
    lyrics
}

fn parse_time_tag(tag: &str) -> Option<f32> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() == 2 {
        let mins: f32 = parts[0].parse().ok()?;
        let secs: f32 = parts[1].parse().ok()?;
        Some(mins * 60.0 + secs)
    } else {
        None
    }
}
