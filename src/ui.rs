use eframe::egui;
use crate::player::MediaPlayer;
use crate::lyrics::{load_lrc, LyricLine};
use std::time::Instant;

pub struct PlaylistItem {
    pub title: String,
    pub path: String,
}

pub struct Playlist {
    pub name: String,
    pub items: Vec<PlaylistItem>,
}

pub struct OrkaPlayerApp {
    player: MediaPlayer,
    repeat: bool,
    shuffle: bool,
    active_tab: String,
    
    // Playlists & IPTV M3U
    playlists: Vec<Playlist>,
    active_playlist_idx: Option<usize>,
    current_playlist_item_idx: Option<usize>,

    // Video / Focus mode (gdy odtwarza, interfejs znika do wciśnięcia Ctrl+K)
    focus_mode: bool,

    // Video placeholder
    video_path: Option<String>,

    // Ekran startowy
    app_start_time: Instant,
    show_splash: bool,

    // Sync Lyrics
    show_sync_window: bool,
    sync_audio_path: Option<String>,
    sync_raw_text: String,
    sync_mode: SyncState,

    // Credits modal
    show_credits_window: bool,

    current_lyrics: Vec<LyricLine>,
    loaded_lyrics_path: Option<String>,
}

#[derive(PartialEq)]
enum SyncState {
    Idle,
    CountingDown(Instant),
    RecordingWordByWord {
        timer: Instant,
        word_index: usize,
        all_words: Vec<String>,
        recorded_data: Vec<(f32, String)>,
    },
    Finished,
}

impl Default for OrkaPlayerApp {
    fn default() -> Self {
        Self {
            player: MediaPlayer::new(),
            repeat: false,
            shuffle: false,
            active_tab: "Audio".to_string(),
            playlists: vec![
                Playlist {
                    name: "Ulubione".to_string(),
                    items: vec![],
                },
                Playlist {
                    name: "IPTV / Streamy M3U".to_string(),
                    items: vec![],
                },
            ],
            active_playlist_idx: None,
            current_playlist_item_idx: None,
            focus_mode: false,
            video_path: None,
            app_start_time: Instant::now(),
            show_splash: true,
            show_sync_window: false,
            sync_audio_path: None,
            sync_raw_text: "Ich muss zerstören alles was du hast".to_string(),
            sync_mode: SyncState::Idle,
            show_credits_window: false,
            current_lyrics: vec![],
            loaded_lyrics_path: None,
        }
    }
}

impl OrkaPlayerApp {
    fn play_next(&mut self) {
        if let Some(p_idx) = self.active_playlist_idx {
            if let Some(item_idx) = self.current_playlist_item_idx {
                let playlist = &self.playlists[p_idx];
                if !playlist.items.is_empty() {
                    let next_idx = if self.shuffle {
                        rand_idx(playlist.items.len())
                    } else {
                        (item_idx + 1) % playlist.items.len()
                    };
                    self.current_playlist_item_idx = Some(next_idx);
                    let path = playlist.items[next_idx].path.clone();
                    self.player.load_and_play(&path);
                }
            }
        }
    }

    fn play_prev(&mut self) {
        if let Some(p_idx) = self.active_playlist_idx {
            if let Some(item_idx) = self.current_playlist_item_idx {
                let playlist = &self.playlists[p_idx];
                if !playlist.items.is_empty() {
                    let prev_idx = if item_idx == 0 { playlist.items.len() - 1 } else { item_idx - 1 };
                    self.current_playlist_item_idx = Some(prev_idx);
                    let path = playlist.items[prev_idx].path.clone();
                    self.player.load_and_play(&path);
                }
            }
        }
    }
}

fn rand_idx(max: usize) -> usize {
    use std::time::SystemTime;
    let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().subsec_nanos();
    (seed as usize) % max
}

impl eframe::App for OrkaPlayerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // Skrót CTRL + K przełącza tryb skupienia (ukrywa interfejs / przywraca panel)
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
            self.focus_mode = !self.focus_mode;
        }

        // --- SPLASH SCREEN ---
        if self.show_splash {
            let elapsed = self.app_start_time.elapsed().as_secs_f32();
            if elapsed < 2.0 {
                ctx.request_repaint();
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(140.0);
                        ui.colored_label(
                            egui::Color32::from_rgb(80, 255, 120),
                            egui::RichText::new("🐋 ORKA PLAYER LINUX").font(egui::FontId::proportional(36.0)).strong(),
                        );
                        ui.add_space(10.0);
                        ui.colored_label(
                            egui::Color32::from_gray(160),
                            egui::RichText::new("Multimedia, IPTV & Smooth Karaoke Engine").font(egui::FontId::proportional(16.0)),
                        );
                        ui.add_space(40.0);
                        ui.spinner();
                    });
                });
                return;
            } else {
                self.show_splash = false;
            }
        }

        // Ładowanie LRC
        if let Some(path) = &self.player.current_path {
            if !path.starts_with("http") && self.loaded_lyrics_path.as_ref() != Some(path) {
                self.current_lyrics = load_lrc(path);
                self.loaded_lyrics_path = Some(path.clone());
            }
        } else {
            if !self.current_lyrics.is_empty() {
                self.current_lyrics.clear();
                self.loaded_lyrics_path = None;
            }
        }

        // Obsługa Repeat
        if self.repeat && !self.player.is_playing && self.player.current_path.is_some() {
            if let Some(path) = self.player.current_path.clone() {
                self.player.load_and_play(&path);
            }
        }

        // --- JEŚLI FOCUS MODE JEST WŁĄCZONY ---
        if self.focus_mode {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(80, 255, 120),
                        egui::RichText::new("🔴 TRYB ODTWARZANIA / IPTV NA ŻYWO").font(egui::FontId::proportional(22.0)).strong(),
                    );
                    ui.add_space(10.0);
                    let current_title = self.player.current_track.clone().unwrap_or_else(|| "Brak aktywnego strumienia".into());
                    ui.colored_label(egui::Color32::WHITE, format!("Odtwarzane: {}", current_title));

                    ui.add_space(30.0);
                    let (response, painter) = ui.allocate_painter(egui::Vec2::new(760.0, 340.0), egui::Sense::hover());
                    painter.rect_filled(response.rect, 0.0, egui::Color32::BLACK);
                    painter.rect_stroke(response.rect, 4.0, egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(60, 200, 100)));

                    painter.text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "📺 Strumień wideo / audio odtwarza w tle\n\nWciśnij [ CTRL + K ], aby przywrócić interfejs\ni zmienić kanał lub playlistę.",
                        egui::FontId::proportional(16.0),
                        egui::Color32::LIGHT_GRAY,
                    );

                    ui.add_space(30.0);
                    ui.horizontal(|ui| {
                        if ui.button("⏸ Pauza / Start").clicked() {
                            self.player.toggle_play_pause();
                        }
                        if ui.button("⏹ Stop").clicked() {
                            self.player.stop();
                        }
                        if ui.button("🎛 Pokaż Playlistę / Interfejs (Ctrl+K)").clicked() {
                            self.focus_mode = false;
                        }
                    });
                });
            });
            return;
        }

        // --- GÓRNY PANEL ---
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                if ui.button("Video").clicked() { self.active_tab = "Video".into(); }
                if ui.button("Audio").clicked() {
                    self.active_tab = "Audio".into();
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Some(path_str) = path.to_str() {
                            self.player.load_and_play(path_str);
                        }
                    }
                }
                if ui.button("Playlist Manager").clicked() { self.active_tab = "Playlist".into(); }
                
                ui.menu_button("Tools", |ui| {
                    if ui.button("Sync Lyrics...").clicked() {
                        self.show_sync_window = true;
                        ui.close_menu();
                    }
                });

                if ui.button("Credits").clicked() {
                    self.show_credits_window = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("OrkaPlayer Linux");
                    ui.checkbox(&mut self.repeat, "Repeat");
                    if ui.button("📺 Focus Mode (Ctrl+K)").clicked() {
                        self.focus_mode = true;
                    }
                });
            });
        });

        // --- DOLNY PANEL STEROWANIA ---
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("|<").clicked() { self.play_prev(); }

                let play_pause_label = if self.player.is_playing { "Pause" } else { "Play" };
                if ui.button(play_pause_label).clicked() { self.player.toggle_play_pause(); }

                if ui.button(">>|").clicked() { self.play_next(); }

                let current_time = self.player.get_current_position();
                let mins = (current_time / 60.0).floor() as u32;
                let secs = (current_time % 60.0).floor() as u32;
                ui.label(format!("{:02}:{:02}", mins, secs));

                let mut progress = current_time;
                if ui.add(egui::Slider::new(&mut progress, 0.0..=self.player.duration).show_value(false)).changed() {}
                ui.label("3:00");

                ui.checkbox(&mut self.shuffle, "Shuffle");
                ui.label("vol");
                let mut vol = self.player.volume;
                if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false)).changed() {
                    self.player.set_volume(vol);
                }
            });
        });

        // --- CENTRALNY PANEL ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_tab == "Video" {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.heading("🎬 Odtwarzacz Wideo");
                    ui.add_space(20.0);
                    if ui.button("Wybierz plik wideo...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.video_path = path.to_str().map(|s| s.to_string());
                        }
                    }
                    ui.add_space(20.0);
                    let v_label = self.video_path.as_deref().unwrap_or("Brak wybranego wideo");
                    ui.colored_label(egui::Color32::LIGHT_BLUE, v_label);
                    
                    ui.add_space(40.0);
                    let (response, painter) = ui.allocate_painter(egui::Vec2::new(700.0, 350.0), egui::Sense::hover());
                    painter.rect_filled(response.rect, 0.0, egui::Color32::BLACK);
                    painter.text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Render wideo / Napisy (Linux Native)",
                        egui::FontId::proportional(16.0),
                        egui::Color32::WHITE,
                    );
                });
            } else if self.active_tab == "Playlist" {
                ui.heading("📁 Playlist Manager & IPTV (M3U)");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Wczytaj plik IPTV M3U...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let mut items = vec![];
                                let mut current_title = "Stream IPTV".to_string();
                                for line in content.lines() {
                                    let line = line.trim();
                                    if line.starts_with("#EXTINF:") {
                                        if let Some(pos) = line.rfind(',') {
                                            current_title = line[pos + 1..].trim().to_string();
                                        }
                                    } else if !line.starts_with('#') && !line.is_empty() {
                                        items.push(PlaylistItem {
                                            title: current_title.clone(),
                                            path: line.to_string(),
                                        });
                                        current_title = "Stream IPTV".to_string();
                                    }
                                }
                                self.playlists.push(Playlist {
                                    name: path.file_name().unwrap().to_str().unwrap().to_string(),
                                    items,
                                });
                            }
                        }
                    }
                });

                ui.add_space(10.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (p_idx, playlist) in self.playlists.iter().enumerate() {
                        ui.collapsing(format!("Playlista: {} ({} pozycji)", playlist.name, playlist.items.len()), |ui| {
                            for (item_idx, item) in playlist.items.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.button("▶ Odtwórz").clicked() {
                                        self.active_playlist_idx = Some(p_idx);
                                        self.current_playlist_item_idx = Some(item_idx);
                                        self.player.load_and_play(&item.path);
                                        self.focus_mode = true;
                                    }
                                    ui.label(&item.title);
                                });
                            }
                        });
                    }
                });
            } else {
                // Audio + Smooth Karaoke
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    if let Some(track) = &self.player.current_track {
                        ui.colored_label(egui::Color32::from_gray(180), format!("Odtwarzane: {}", track));
                        ui.add_space(25.0);

                        let current_time = self.player.get_current_position();

                        let mut active_index = 0;
                        for (i, lyric) in self.current_lyrics.iter().enumerate() {
                            if current_time >= lyric.timestamp {
                                active_index = i;
                            } else {
                                break;
                            }
                        }

                        let start_history = active_index.saturating_sub(2);
                        for i in start_history..active_index {
                            ui.colored_label(
                                egui::Color32::from_gray(80), 
                                egui::RichText::new(&self.current_lyrics[i].text).size(15.0)
                            );
                            ui.add_space(6.0);
                        }

                        if !self.current_lyrics.is_empty() {
                            ui.add_space(4.0);
                            let active_text = &self.current_lyrics[active_index].text;
                            let time_progress = (current_time * 3.0).fract(); 
                            let anim_offset = (time_progress * 4.0).sin() * 1.0;

                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - (active_text.len() as f32 * 12.0)).max(0.0) / 2.0);
                                for (w_idx, word) in active_text.split_whitespace().enumerate() {
                                    let is_current_word = w_idx == (active_index % active_text.split_whitespace().count().max(1));
                                    let color = if is_current_word {
                                        let g = 200 + ((current_time * 8.0).sin() * 55.0) as u8;
                                        egui::Color32::from_rgb(60, g, 120)
                                    } else {
                                        egui::Color32::from_gray(180)
                                    };
                                    let font_size = if is_current_word { 26.0 + anim_offset } else { 22.0 };
                                    ui.colored_label(color, egui::RichText::new(word).size(font_size).strong());
                                }
                            });
                            ui.add_space(8.0);
                        } else {
                            ui.colored_label(
                                egui::Color32::YELLOW, 
                                egui::RichText::new("Brak pliku .lrc. Stwórz go w Tools -> Sync Lyrics!").size(18.0)
                            );
                        }

                        if !self.current_lyrics.is_empty() {
                            let end_future = (active_index + 3).min(self.current_lyrics.len());
                            for i in (active_index + 1)..end_future {
                                let opacity = 140 - ((i - active_index) * 35) as u8;
                                ui.colored_label(
                                    egui::Color32::from_gray(opacity), 
                                    egui::RichText::new(&self.current_lyrics[i].text).size(15.0)
                                );
                                ui.add_space(6.0);
                            }
                        }

                        ctx.request_repaint();

                        ui.add_space(25.0);
                        ui.separator();
                        ui.label("Prawdziwy oscyloskop audio");

                        let (response, painter) = ui.allocate_painter(egui::Vec2::new(780.0, 90.0), egui::Sense::hover());
                        let rect = response.rect;
                        painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

                        if let Ok(buffer) = self.player.waveform_data.lock() {
                            let mut points = vec![];
                            let width = rect.width();
                            let height = rect.height();
                            let len = buffer.len();
                            if len > 1 {
                                for (i, &sample) in buffer.iter().enumerate() {
                                    let x = rect.min.x + (i as f32 / len as f32) * width;
                                    let y = rect.center().y - (sample * (height * 0.4));
                                    points.push(egui::pos2(x, y));
                                }
                                painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(80, 255, 120))));
                            }
                        }
                    } else {
                        ui.add_space(80.0);
                        ui.heading("OrkaPlayer Linux v2.4");
                        ui.add_space(20.0);
                        ui.colored_label(egui::Color32::from_gray(140), "Wybierz plik [Audio] u góry lub utwórz playlistę.");
                    }
                });
            }
        });

        // --- OKNO CREDITS ---
        if self.show_credits_window {
            let mut open = true;
            egui::Window::new("OrkaPlayer - Credits")
                .open(&mut open)
                .default_size([450.0, 300.0])
                .show(ctx, |ui| {
                    ui.heading("OrkaPlayer Linux");
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Oryginalny projekt: ");
                        if ui.hyperlink_to("cyz-yt", "https://github.com/cyz-yt/").clicked() {}
                    });
                    
                    ui.label("Port napisany przez: ridikhost");
                    ui.label("Port napisany w: Rust");
                    ui.add_space(10.0);
                    
                    ui.label("Użyte biblioteki:");
                    ui.label("• eframe / egui (GUI Framework)");
                    ui.label("• rodio (Audio Engine & PCM Streaming)");
                    ui.label("• rfd (Native File Dialogs)");
                    ui.label("• tokio & reqwest (Async / Networking)");
                    ui.label("• serde & serde_json (Serialization)");
                });
            if !open {
                self.show_credits_window = false;
            }
        }

        // --- OKNO SYNC LYRICS ---
        if self.show_sync_window {
            let mut open = true;
            egui::Window::new("OrkaPlayer - Sync Lyrics")
                .open(&mut open)
                .default_size([650.0, 480.0])
                .show(ctx, |ui| {
                    ui.heading("Synchronizacja tekstów");
                    ui.label("Wklej tekst, wybierz plik piosenki, kliknij start i wciskaj Spację na każde słowo!");
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Wybierz plik muzyczny...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                self.sync_audio_path = path.to_str().map(|s| s.to_string());
                            }
                        }
                        let file_label = self.sync_audio_path.as_deref().unwrap_or("Brak wybranego pliku");
                        ui.colored_label(egui::Color32::LIGHT_BLUE, file_label);
                    });

                    ui.add_space(10.0);
                    ui.label("Tekst piosenki:");
                    ui.add_sized([630.0, 120.0], egui::TextEdit::multiline(&mut self.sync_raw_text));

                    ui.add_space(15.0);

                    match &mut self.sync_mode {
                        SyncState::Idle => {
                            if ui.button("Zacznij dodawać (Start)").clicked() {
                                if let Some(path) = &self.sync_audio_path {
                                    self.player.load_and_play(path);
                                    self.player.toggle_play_pause();
                                    self.sync_mode = SyncState::CountingDown(Instant::now());
                                }
                            }
                        }
                        SyncState::CountingDown(start_time) => {
                            let elapsed = start_time.elapsed().as_secs_f32();
                            let countdown = 5.0 - elapsed;
                            if countdown > 0.0 {
                                ui.colored_label(egui::Color32::YELLOW, format!("Start za: {:.1} sekund...", countdown));
                                ctx.request_repaint();
                            } else {
                                self.player.toggle_play_pause();
                                let all_words: Vec<String> = self.sync_raw_text
                                    .split_whitespace()
                                    .map(|s| s.to_string())
                                    .collect();

                                self.sync_mode = SyncState::RecordingWordByWord {
                                    timer: Instant::now(),
                                    word_index: 0,
                                    all_words,
                                    recorded_data: vec![],
                                };
                            }
                        }
                        SyncState::RecordingWordByWord { word_index, all_words, recorded_data, .. } => {
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::GREEN, "▶ NAGRYWANIE SYNCHRONIZACJI");
                                ui.label(format!("Słowo {} / {}", *word_index + 1, all_words.len()));
                            });

                            if *word_index < all_words.len() {
                                ui.add_space(15.0);
                                ui.colored_label(egui::Color32::LIGHT_GREEN, format!("NASTĘPNE SŁOWO: [ {} ]", all_words[*word_index]));
                                ui.add_space(10.0);
                                ui.label("Wciśnij [ SPACE ] w momencie śpiewania tego słowa!");

                                if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                                    let time_offset = self.player.get_current_position();
                                    recorded_data.push((time_offset, all_words[*word_index].clone()));
                                    *word_index += 1;
                                }
                                ctx.request_repaint();
                            } else {
                                if let Some(path) = &self.sync_audio_path {
                                    self.player.save_word_sync_lrc(path, recorded_data);
                                }
                                self.player.stop();
                                self.sync_mode = SyncState::Finished;
                            }
                        }
                        SyncState::Finished => {
                            ui.colored_label(egui::Color32::GREEN, "Gotowe! Plik LRC został pomyślnie zapisany obok piosenki.");
                            if ui.button("Zamknij okno").clicked() {
                                self.sync_mode = SyncState::Idle;
                                self.show_sync_window = false;
                            }
                        }
                    }
                });
            if !open {
                self.show_sync_window = false;
            }
        }
    }
}

pub fn run_ui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 560.0])
            .with_title("OrkaPlayer Linux v2.4"),
        ..Default::default()
    };
    eframe::run_native("OrkaPlayer Linux", options, Box::new(|_cc| Ok(Box::new(OrkaPlayerApp::default()))))
}
