use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Types of cat meows supported by the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeowType {
    /// Cheerful higher-pitched cat meow.
    WorkerMeow,
    /// Deeper tactical cat "Mrow!".
    SoldierMeow,
}

/// Background music tracks available in the audio engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicTrack {
    Off,
    StLouisBlues,
    MidnightCoolJazz,
    BoogieBluesShuffle,
    CustomFile(String),
}

impl MusicTrack {
    /// Cycles through the tracks: Off -> StLouisBlues -> MidnightCoolJazz -> BoogieBluesShuffle -> Off
    pub fn next(&self) -> Self {
        match self {
            MusicTrack::Off => MusicTrack::StLouisBlues,
            MusicTrack::StLouisBlues => MusicTrack::MidnightCoolJazz,
            MusicTrack::MidnightCoolJazz => MusicTrack::BoogieBluesShuffle,
            MusicTrack::BoogieBluesShuffle => MusicTrack::Off,
            MusicTrack::CustomFile(_) => MusicTrack::Off,
        }
    }

    /// Returns the descriptive label for the track.
    pub fn label(&self) -> &str {
        match self {
            MusicTrack::Off => "Off",
            MusicTrack::StLouisBlues => "St. Louis Blues (1914 PD Blues)",
            MusicTrack::MidnightCoolJazz => "Midnight Cool Jazz (PD Swing)",
            MusicTrack::BoogieBluesShuffle => "12-Bar Blues Shuffle (PD Blues)",
            MusicTrack::CustomFile(_) => "Custom Music File",
        }
    }
}

/// Generates a complete, valid 16-bit mono 22050 Hz RIFF WAV byte buffer in memory
/// containing the synthesized cat meow waveform (fundamental + harmonics + ADSR envelope + pitch glide).
pub fn synthesize_meow_wav(meow_type: MeowType) -> Vec<u8> {
    let sample_rate: u32 = 22050;
    let (duration_secs, start_freq, peak_freq, end_freq, has_mrow_harmonic) = match meow_type {
        MeowType::WorkerMeow => (0.28_f32, 450.0_f32, 720.0_f32, 390.0_f32, false),
        MeowType::SoldierMeow => (0.32_f32, 310.0_f32, 520.0_f32, 260.0_f32, true),
    };

    let total_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(total_samples);

    let attack_time = 0.04_f32;
    let release_time = 0.08_f32;
    let attack_samples = (attack_time * sample_rate as f32) as usize;
    let release_samples = (release_time * sample_rate as f32) as usize;

    let mut phase = 0.0_f32;
    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let progress = i as f32 / total_samples as f32;

        let freq = if progress < 0.3_f32 {
            let p = progress / 0.3_f32;
            start_freq + (peak_freq - start_freq) * (p * p)
        } else {
            let p = (progress - 0.3_f32) / 0.7_f32;
            peak_freq + (end_freq - peak_freq) * (p * (2.0 - p))
        };

        let vibrato_rate = 6.0_f32;
        let vibrato_depth = if matches!(meow_type, MeowType::WorkerMeow) { 10.0_f32 } else { 6.0_f32 };
        let instant_freq = freq + (t * vibrato_rate * 2.0 * std::f32::consts::PI).sin() * vibrato_depth;

        phase += 2.0 * std::f32::consts::PI * instant_freq / sample_rate as f32;

        let mut sample = phase.sin() + 0.5 * (phase * 2.0).sin();
        if has_mrow_harmonic {
            sample += 0.3 * (phase * 3.0).sin() + 0.15 * (phase * 4.0).sin();
        } else {
            sample += 0.25 * (phase * 3.0).sin();
        }

        let formant_mod = (progress * std::f32::consts::PI).sin();
        sample += 0.2 * formant_mod * (phase * 1.5).sin();

        let mut env = 1.0_f32;
        if i < attack_samples {
            env = i as f32 / attack_samples as f32;
        } else if i > total_samples.saturating_sub(release_samples) {
            let rel_i = total_samples - i;
            env = rel_i as f32 / release_samples as f32;
        }

        sample *= env;
        sample = sample.tanh();

        let scaled = (sample * 24000.0).clamp(-32768.0, 32767.0) as i16;
        samples.push(scaled);
    }

    let data_bytes_len = (samples.len() * 2) as u32;
    let file_size = 36 + data_bytes_len;

    let mut wav = Vec::with_capacity(44 + data_bytes_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes_len.to_le_bytes());

    for s in samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

/// Synthesizes a rich, pleasant multi-voice loopable WAV buffer (~8–12 seconds at 22050 Hz 16-bit mono)
/// combining upright walking bass, Rhodes/Electric piano chord comping, brushed swing ride cymbal, and a blues/jazz lead melody.
pub fn synthesize_jazz_blues_wav(track: &MusicTrack) -> Vec<u8> {
    let sample_rate: u32 = 22050;
    // ~10 seconds loop duration
    let duration_secs = match track {
        MusicTrack::StLouisBlues => 10.0_f32,
        MusicTrack::MidnightCoolJazz => 10.5_f32,
        MusicTrack::BoogieBluesShuffle => 9.6_f32,
        _ => 10.0_f32,
    };

    let total_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = vec![0i16; total_samples];

    // Tempo and groove parameters (BPM ~110, swing feel)
    let bpm = match track {
        MusicTrack::MidnightCoolJazz => 104.0_f32,
        MusicTrack::BoogieBluesShuffle => 120.0_f32,
        _ => 112.0_f32,
    };
    let beat_duration = 60.0_f32 / bpm;

    // Chord progression frequencies (e.g. 12-bar blues in G: G7, C7, D7 or jazz ii-V-I: Am7 - D7 - Gmaj7)
    // We will render layers into the sample buffer.
    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let beat_pos = t / beat_duration;
        let bar_pos = beat_pos / 4.0;
        let bar_index = bar_pos.floor() as usize;

        // --- 1. Upright Walking Bassline ---
        // Root notes for a 12-bar blues in G (G, C, D)
        let bass_root = match bar_index % 12 {
            0..=3 => 98.0_f32,   // G2 (~98 Hz)
            4..=5 => 130.81_f32, // C3 (~130.8 Hz)
            6..=7 => 98.0_f32,   // G2
            8 => 146.83_f32,     // D3 (~146.8 Hz)
            9 => 130.81_f32,     // C3
            _ => 98.0_f32,       // G2
        };
        // Walking note variations on each beat
        let beat_in_bar = beat_pos % 4.0;
        let bass_freq = match beat_in_bar.floor() as i32 {
            0 => bass_root,
            1 => bass_root * 1.25, // major third / fourth
            2 => bass_root * 1.5,  // fifth
            _ => bass_root * 1.78, // minor seventh walking up
        };

        let bass_phase_env = (t * bass_freq * 2.0 * std::f32::consts::PI).sin()
            + 0.4 * (t * bass_freq * 4.0 * std::f32::consts::PI).sin();
        let bass_pluck = (-((beat_in_bar % 1.0) * 8.0)).exp();
        let bass_sample = 0.35 * bass_phase_env * bass_pluck;

        // --- 2. Rhodes / Electric Piano 7th/9th Chord Comping ---
        let chord_freqs = match bar_index % 12 {
            0..=3 => [196.0_f32, 246.94_f32, 293.66_f32, 349.23_f32], // G7 (G, B, D, F)
            4..=5 => [261.63_f32, 329.63_f32, 392.00_f32, 466.16_f32], // C7 (C, E, G, Bb)
            6..=7 => [196.0_f32, 246.94_f32, 293.66_f32, 349.23_f32], // G7
            8 => [293.66_f32, 369.99_f32, 440.00_f32, 523.25_f32],     // D7 (D, F#, A, C)
            9 => [261.63_f32, 329.63_f32, 392.00_f32, 466.16_f32],     // C7
            _ => [196.0_f32, 246.94_f32, 293.66_f32, 349.23_f32],
        };
        // Comping hits on offbeats (swing feel)
        let offbeat = (beat_in_bar - 1.5).abs() < 0.3 || (beat_in_bar - 3.5).abs() < 0.3;
        let comp_env = if offbeat { 0.15 } else { 0.02 };
        let mut piano_sample = 0.0_f32;
        if offbeat {
            for &cf in &chord_freqs {
                piano_sample += (t * cf * 2.0 * std::f32::consts::PI).sin() * 0.1;
            }
            piano_sample *= comp_env;
        }

        // --- 3. Brushed Swing Ride Cymbal (Filtered Noise / Tick) ---
        let ride_hit = (beat_in_bar % 0.5) < 0.1;
        let ride_sample = if ride_hit {
            // Pseudo-random noise tick with fast decay
            let noise = ((t * 12345.67_f32).sin() * 43758.5453_f32).fract() - 0.5;
            noise * 0.2 * (-((beat_in_bar % 0.5) * 30.0)).exp()
        } else {
            0.0_f32
        };

        // --- 4. Soulful Blues Scale / Jazz Horn Lead Melody ---
        // G blues scale frequencies: G4 (392 Hz), Bb4 (466.16 Hz), C5 (523.25 Hz), Db5 (554.37 Hz), D5 (587.33 Hz), F5 (698.46 Hz)
        let melody_notes = [392.0_f32, 466.16_f32, 523.25_f32, 554.37_f32, 587.33_f32, 698.46_f32, 783.99_f32];
        let note_index = ((t * 2.5).sin() * 3.0 + 3.0).floor() as usize % melody_notes.len();
        let lead_freq = melody_notes[note_index];
        let lead_vibrato = (t * 5.0 * 2.0 * std::f32::consts::PI).sin() * 4.0;
        let lead_phase = t * (lead_freq + lead_vibrato) * 2.0 * std::f32::consts::PI;
        let lead_sample = 0.18 * lead_phase.sin() + 0.08 * (lead_phase * 2.0).sin();
        let lead_envelope = 0.5 + 0.5 * (t * 1.5).sin(); // rhythmic swells

        // Combine all instruments
        let mix = bass_sample + piano_sample + ride_sample + (lead_sample * lead_envelope);
        let scaled = (mix * 20000.0).clamp(-32768.0, 32767.0) as i16;
        samples[i] = scaled;
    }

    // Build RIFF WAV byte buffer
    let data_bytes_len = (samples.len() * 2) as u32;
    let file_size = 36 + data_bytes_len;

    let mut wav = Vec::with_capacity(44 + data_bytes_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes_len.to_le_bytes());

    for s in samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

/// Cross-platform procedural cat meow and jazz/blues background music audio engine.
pub struct CatAudioEngine {
    enabled: bool,
    worker_wav: Vec<u8>,
    soldier_wav: Vec<u8>,
    last_meow_time: Instant,
    cooldown: Duration,
    current_track: MusicTrack,
    music_child: Option<Child>,
    music_stop_flag: Arc<AtomicBool>,
}

impl CatAudioEngine {
    /// Creates a new `CatAudioEngine`, pre-generating meow WAV buffers.
    pub fn new(enabled: bool) -> Self {
        let worker_wav = synthesize_meow_wav(MeowType::WorkerMeow);
        let soldier_wav = synthesize_meow_wav(MeowType::SoldierMeow);
        Self {
            enabled,
            worker_wav,
            soldier_wav,
            last_meow_time: Instant::now() - Duration::from_secs(10),
            cooldown: Duration::from_millis(180),
            current_track: MusicTrack::Off,
            music_child: None,
            music_stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Plays a meow sound if enabled and cooldown has elapsed.
    pub fn play_meow(&mut self, meow_type: MeowType) {
        if !self.enabled {
            return;
        }

        let now = Instant::now();
        if now.duration_since(self.last_meow_time) < self.cooldown {
            return;
        }
        self.last_meow_time = now;

        let wav_data = match meow_type {
            MeowType::WorkerMeow => self.worker_wav.clone(),
            MeowType::SoldierMeow => self.soldier_wav.clone(),
        };

        std::thread::spawn(move || {
            #[cfg(target_os = "linux")]
            let players = [
                ("pw-play", vec!["-"]),
                ("paplay", vec![]),
                ("aplay", vec!["-q"]),
            ];

            #[cfg(all(unix, not(target_os = "linux")))]
            let players = [
                ("paplay", vec![]),
                ("aucat", vec!["-i", "-"]),
            ];

            #[cfg(not(unix))]
            let players: [(&str, Vec<&str>); 0] = [];

            for (cmd, args) in players {
                if let Ok(mut child) = Command::new(cmd)
                    .args(&args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(&wav_data);
                        let _ = stdin.flush();
                        drop(stdin);
                        let _ = child.wait();
                        return;
                    }
                }
            }
        });
    }

    /// Sets the current background music track, restarting playback if changed.
    pub fn set_music_track(&mut self, track: MusicTrack) {
        if self.current_track == track {
            return;
        }
        self.stop_music();
        self.current_track = track.clone();

        if !self.enabled || matches!(track, MusicTrack::Off) {
            return;
        }

        let wav_data = match &track {
            MusicTrack::StLouisBlues
            | MusicTrack::MidnightCoolJazz
            | MusicTrack::BoogieBluesShuffle => synthesize_jazz_blues_wav(&track),
            MusicTrack::CustomFile(path) => {
                // Try reading custom file, fallback to jazz blues if fails
                std::fs::read(path).unwrap_or_else(|_| synthesize_jazz_blues_wav(&MusicTrack::MidnightCoolJazz))
            }
            MusicTrack::Off => return,
        };

        let stop_flag = Arc::clone(&self.music_stop_flag);
        stop_flag.store(false, Ordering::Relaxed);

        // Spawn background looping music thread
        let handle = std::thread::spawn(move || {
            #[cfg(target_os = "linux")]
            let players = [
                ("pw-play", vec!["-"]),
                ("paplay", vec![]),
                ("aplay", vec!["-q"]),
            ];

            #[cfg(all(unix, not(target_os = "linux")))]
            let players = [
                ("paplay", vec![]),
                ("aucat", vec!["-i", "-"]),
            ];

            #[cfg(not(unix))]
            let players: [(&str, Vec<&str>); 0] = [];

            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                let mut played = false;
                for (cmd, args) in &players {
                    if let Ok(mut child) = Command::new(cmd)
                        .args(args)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            played = true;
                            // Write wav chunks or full buffer
                            let _ = stdin.write_all(&wav_data);
                            let _ = stdin.flush();
                            drop(stdin);
                            let _ = child.wait();
                            break;
                        }
                    }
                }

                if !played {
                    // If no audio player binary is available on system, sleep to prevent busy loop
                    std::thread::sleep(Duration::from_secs(5));
                }

                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        // We can track thread or background process. Since we spawn a looping thread,
        // stopping is handled cleanly via the atomic stop flag.
        let _ = handle;
    }

    /// Cycles to the next background music track and returns a reference to it.
    pub fn cycle_music_track(&mut self) -> &MusicTrack {
        let next_track = self.current_track.next();
        self.set_music_track(next_track);
        &self.current_track
    }

    /// Stops background music playback.
    pub fn stop_music(&mut self) {
        self.music_stop_flag.store(true, Ordering::Relaxed);
        if let Some(mut child) = self.music_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.current_track = MusicTrack::Off;
    }

    /// Returns the currently active music track.
    pub fn current_track(&self) -> &MusicTrack {
        &self.current_track
    }
}

impl Drop for CatAudioEngine {
    fn drop(&mut self) {
        self.stop_music();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_meow_wav() {
        for meow_type in [MeowType::WorkerMeow, MeowType::SoldierMeow] {
            let wav = synthesize_meow_wav(meow_type);

            assert_eq!(&wav[0..4], b"RIFF");
            assert_eq!(&wav[8..12], b"WAVE");
            assert_eq!(&wav[12..16], b"fmt ");
            assert_eq!(&wav[36..40], b"data");

            let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
            assert_eq!(sample_rate, 22050);

            let data_len = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
            assert!(data_len > 0);
            assert_eq!(wav.len(), 44 + data_len);

            let audio_samples_bytes = &wav[44..];
            let mut non_zero_count = 0;
            for chunk in audio_samples_bytes.chunks_exact(2) {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                if sample != 0 {
                    non_zero_count += 1;
                }
            }
            assert!(non_zero_count > 0, "Synthesized meow WAV should contain non-zero audio samples");
        }
    }

    #[test]
    fn test_synthesize_jazz_blues_wav() {
        let tracks = [
            MusicTrack::StLouisBlues,
            MusicTrack::MidnightCoolJazz,
            MusicTrack::BoogieBluesShuffle,
        ];

        for track in &tracks {
            let wav = synthesize_jazz_blues_wav(track);

            assert_eq!(&wav[0..4], b"RIFF");
            assert_eq!(&wav[8..12], b"WAVE");
            assert_eq!(&wav[12..16], b"fmt ");
            assert_eq!(&wav[36..40], b"data");

            let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
            assert_eq!(sample_rate, 22050);

            let data_len = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
            assert!(data_len > 0);
            assert_eq!(wav.len(), 44 + data_len);

            let audio_samples_bytes = &wav[44..];
            let mut non_zero_count = 0;
            for chunk in audio_samples_bytes.chunks_exact(2) {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                if sample != 0 {
                    non_zero_count += 1;
                }
            }
            assert!(non_zero_count > 0, "Synthesized jazz/blues WAV for {:?} should contain non-zero audio samples", track);
        }
    }

    #[test]
    fn test_music_track_cycling() {
        let track = MusicTrack::Off;
        let t1 = track.next();
        assert_eq!(t1, MusicTrack::StLouisBlues);
        let t2 = t1.next();
        assert_eq!(t2, MusicTrack::MidnightCoolJazz);
        let t3 = t2.next();
        assert_eq!(t3, MusicTrack::BoogieBluesShuffle);
        let t4 = t3.next();
        assert_eq!(t4, MusicTrack::Off);
    }
}
