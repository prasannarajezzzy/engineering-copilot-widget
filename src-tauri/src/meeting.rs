use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use wasapi::{
    AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat,
};

const MAX_SAMPLES: usize = 16_000 * 600;
const SAMPLE_RATE: u32 = 16_000;

#[derive(Serialize, Deserialize, Clone)]
pub struct MeetingNote {
    pub id: String,
    pub created: i64,
    pub decisions: Vec<String>,
    pub actions: Vec<String>,
    pub recap: String,
    pub heard: String,
    pub transcript: String,
    pub trimmed: bool,
}

#[derive(Serialize)]
pub struct ListenStatus {
    pub heard: String,
}

struct Session {
    stop: Arc<AtomicBool>,
    mic: Arc<Mutex<Vec<f32>>>,
    room: Arc<Mutex<Vec<f32>>>,
    heard: String,
    capped: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn lock_session() -> std::sync::MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner())
}

fn openai_key() -> Result<String, String> {
    let key = crate::key_entry("openai")?
        .get_password()
        .map_err(|_| "Save an OpenAI key in AI Setup. I use it to turn the meeting into words. Your other model can still write the notes.".to_string())?;
    if key.trim().is_empty() || key == "NO_KEY" {
        return Err("Save an OpenAI key in AI Setup. I use it to turn the meeting into words. Your other model can still write the notes.".into());
    }
    Ok(key)
}

fn notes_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("meetings.json"))
}

fn load_notes(app: &AppHandle) -> Result<Vec<MeetingNote>, String> {
    let path = notes_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("I couldn't read your saved notes ({e})."))
}

fn save_notes(app: &AppHandle, notes: &[MeetingNote]) -> Result<(), String> {
    let path = notes_path(app)?;
    let bytes = serde_json::to_vec_pretty(notes).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

struct Stopper<'a>(&'a AudioClient);
impl Drop for Stopper<'_> {
    fn drop(&mut self) {
        let _ = self.0.stop_stream();
    }
}

fn com_ok() -> Result<(), String> {
    let hr = wasapi::initialize_mta();
    if hr.is_ok() {
        Ok(())
    } else {
        Err(format!("Could not start Windows audio ({hr})."))
    }
}

fn open_client(device_direction: Direction) -> Result<(AudioClient, WaveFormat), String> {
    let label = match device_direction {
        Direction::Capture => "microphone",
        Direction::Render => "meeting audio",
    };
    com_ok()?;
    let enumerator = DeviceEnumerator::new()
        .map_err(|e| format!("I couldn't open the {label} ({e})."))?;
    let device = enumerator
        .get_default_device(&device_direction)
        .map_err(|_| format!("I couldn't find the {label}."))?;
    let wanted = WaveFormat::new(32, 32, &SampleType::Float, 16_000, 1, None);
    let shared = |auto| StreamMode::PollingShared {
        autoconvert: auto,
        buffer_duration_hns: 1_000_000,
    };
    let mut client = device
        .get_iaudioclient()
        .map_err(|e| format!("I couldn't open the {label} ({e})."))?;
    if client
        .initialize_client(&wanted, &Direction::Capture, &shared(true))
        .is_ok()
    {
        return Ok((client, wanted));
    }
    let mut client = device
        .get_iaudioclient()
        .map_err(|e| format!("I couldn't open the {label} ({e})."))?;
    let mix = client
        .get_mixformat()
        .map_err(|e| format!("I couldn't use the {label} ({e})."))?;
    client
        .initialize_client(&mix, &Direction::Capture, &shared(false))
        .map_err(|e| format!("I couldn't use the {label} ({e})."))?;
    Ok((client, mix))
}

fn decode(bytes: &[u8], kind: SampleType, bits: u16) -> f32 {
    let sample = match (kind, bits) {
        (SampleType::Float, 32) => {
            let mut buf = [0u8; 4];
            let n = bytes.len().min(4);
            buf[..n].copy_from_slice(&bytes[..n]);
            f32::from_le_bytes(buf)
        }
        (SampleType::Float, 64) => {
            let mut buf = [0u8; 8];
            let n = bytes.len().min(8);
            buf[..n].copy_from_slice(&bytes[..n]);
            f64::from_le_bytes(buf) as f32
        }
        (SampleType::Int, 16) => {
            let mut buf = [0u8; 2];
            let n = bytes.len().min(2);
            buf[..n].copy_from_slice(&bytes[..n]);
            i16::from_le_bytes(buf) as f32 / 32768.0
        }
        (SampleType::Int, 32) => {
            let mut buf = [0u8; 4];
            let n = bytes.len().min(4);
            buf[..n].copy_from_slice(&bytes[..n]);
            i32::from_le_bytes(buf) as f32 / 2147483648.0
        }
        (SampleType::Int, 8) => {
            let v = bytes.first().copied().unwrap_or(128);
            (v as f32 - 128.0) / 128.0
        }
        _ => 0.0,
    };
    if sample.is_finite() { sample } else { 0.0 }
}

fn push_frames(out: &mut Vec<f32>, bytes: &[u8], frames: usize, channels: usize, kind: SampleType, bits: u16) {
    let width = (bits as usize / 8).max(1);
    for i in 0..frames {
        let mut sum = 0.0f32;
        let mut count = 0usize;
        for c in 0..channels {
            let start = (i * channels + c) * width;
            if start >= bytes.len() {
                break;
            }
            sum += decode(&bytes[start..], kind, bits);
            count += 1;
        }
        if count > 0 {
            out.push(sum / count as f32);
        }
    }
}

struct Resampler {
    from: u32,
    next: f64,
    prev: Vec<f32>,
}

impl Resampler {
    fn new(from: u32) -> Self {
        Self { from, next: 0.0, prev: Vec::new() }
    }

    fn push(&mut self, frames: &[f32], out: &mut Vec<f32>) {
        if self.from == 0 || frames.is_empty() {
            return;
        }
        if self.from == SAMPLE_RATE {
            out.extend_from_slice(frames);
            return;
        }
        let mut input = std::mem::take(&mut self.prev);
        input.extend_from_slice(frames);
        let step = self.from as f64 / SAMPLE_RATE as f64;
        while self.next + 1.0 < input.len() as f64 {
            let j = self.next.floor() as usize;
            let frac = (self.next - j as f64) as f32;
            let a = input[j];
            let b = input[j + 1];
            let sample = a + (b - a) * frac;
            out.push(if sample.is_finite() { sample } else { 0.0 });
            self.next += step;
        }
        let keep_from = self.next.floor() as usize;
        self.next -= keep_from as f64;
        if keep_from < input.len() {
            self.prev = input[keep_from..].to_vec();
        }
    }
}

fn capture_loop(
    device_direction: Direction,
    stop: Arc<AtomicBool>,
    capped: Arc<AtomicBool>,
    out: Arc<Mutex<Vec<f32>>>,
    started: &Sender<Result<(), String>>,
) -> Result<(), String> {
    let (client, format) = open_client(device_direction)?;
    let channels = format.get_nchannels() as usize;
    let bits = format.get_bitspersample();
    let block = format.get_blockalign() as usize;
    let rate = format.get_samplespersec();
    let kind = format.get_subformat().map_err(|e| e.to_string())?;
    if channels == 0 || block == 0 || rate == 0 {
        return Err("The audio device gave me a format I can't use.".into());
    }
    client.start_stream().map_err(|e| format!("I couldn't start listening ({e})."))?;
    let _stop_stream = Stopper(&client);
    let capture = client
        .get_audiocaptureclient()
        .map_err(|e| format!("I couldn't start listening ({e})."))?;
    let _ = started.send(Ok(()));
    let mut resampler = Resampler::new(rate);
    let mut buf = vec![0u8; 65_536];
    while !stop.load(Ordering::Relaxed) && !capped.load(Ordering::Relaxed) {
        let packet = match capture.get_next_packet_size() {
            Ok(value) => value.unwrap_or(0) as usize,
            Err(_) => break,
        };
        if packet == 0 {
            thread::sleep(Duration::from_millis(12));
            continue;
        }
        let need = packet.saturating_mul(block);
        if need == 0 {
            continue;
        }
        if buf.len() < need {
            buf.resize(need, 0);
        }
        let (frames, info) = match capture.read_from_device(&mut buf[..need]) {
            Ok(value) => value,
            Err(wasapi::WasapiError::DataLengthTooShort { expected, .. }) => {
                let bigger = expected.saturating_mul(block.max(1));
                if buf.len() < bigger {
                    buf.resize(bigger, 0);
                }
                continue;
            }
            Err(_) => break,
        };
        let frames = frames as usize;
        if frames == 0 {
            continue;
        }
        let mut mono = Vec::with_capacity(frames);
        if info.flags.silent {
            mono.resize(frames, 0.0);
        } else {
            let used = frames.saturating_mul(block).min(need);
            push_frames(&mut mono, &buf[..used], frames, channels, kind, bits);
        }
        let mut resampled = Vec::new();
        resampler.push(&mono, &mut resampled);
        let mut guard = out.lock().unwrap_or_else(|e| e.into_inner());
        if guard.len() >= MAX_SAMPLES {
            capped.store(true, Ordering::Relaxed);
            break;
        }
        let room = MAX_SAMPLES - guard.len();
        if resampled.len() > room {
            guard.extend_from_slice(&resampled[..room]);
            capped.store(true, Ordering::Relaxed);
            break;
        }
        guard.extend(resampled);
    }
    Ok(())
}

fn spawn_capture(
    device_direction: Direction,
    stop: Arc<AtomicBool>,
    capped: Arc<AtomicBool>,
) -> (JoinHandle<()>, mpsc::Receiver<Result<(), String>>, Arc<Mutex<Vec<f32>>>) {
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let (tx, rx) = mpsc::channel();
    let samples_thread = samples.clone();
    let handle = thread::spawn(move || {
        if let Err(error) = capture_loop(device_direction, stop, capped, samples_thread, &tx) {
            let _ = tx.send(Err(error));
        }
    });
    (handle, rx, samples)
}

fn side_result(rx: &mpsc::Receiver<Result<(), String>>) -> Result<(), String> {
    match rx.recv_timeout(Duration::from_secs(4)) {
        Ok(result) => result,
        Err(_) => Err("The audio device took too long to wake up.".into()),
    }
}

#[tauri::command]
pub fn start_meeting(_app: AppHandle) -> Result<ListenStatus, String> {
    let _key = openai_key()?;
    let mut slot = lock_session();
    if slot.is_some() {
        return Err("I'm already listening.".into());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let capped = Arc::new(AtomicBool::new(false));
    let (mic_handle, mic_rx, mic) = spawn_capture(Direction::Capture, stop.clone(), capped.clone());
    let (room_handle, room_rx, room) = spawn_capture(Direction::Render, stop.clone(), capped.clone());
    let mic_result = side_result(&mic_rx);
    let room_result = side_result(&room_rx);
    let mic_ok = mic_result.is_ok();
    let room_ok = room_result.is_ok();
    if !mic_ok && !room_ok {
        stop.store(true, Ordering::Relaxed);
        let _ = mic_handle.join();
        let _ = room_handle.join();
        let mic_err = mic_result.err().unwrap_or_else(|| "microphone didn't start".into());
        let room_err = room_result.err().unwrap_or_else(|| "meeting audio didn't start".into());
        return Err(format!(
            "I couldn't hear the microphone or the meeting. Allow desktop apps to use the microphone in Windows privacy settings, then try again.\n{mic_err}\n{room_err}"
        ));
    }
    let heard = match (mic_ok, room_ok) {
        (true, true) => "Hearing you and the call.".to_string(),
        (true, false) => format!(
            "Hearing you. The call audio didn't open ({}).",
            room_result.err().unwrap_or_default()
        ),
        (false, true) => format!(
            "Hearing the call. Your microphone didn't open ({}).",
            mic_result.err().unwrap_or_default()
        ),
        (false, false) => unreachable!(),
    };
    *slot = Some(Session {
        stop,
        mic,
        room,
        heard: heard.clone(),
        capped,
        handles: vec![mic_handle, room_handle],
    });
    Ok(ListenStatus { heard })
}

fn take_samples(samples: &Mutex<Vec<f32>>) -> Vec<f32> {
    let mut guard = samples.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *guard)
}

fn mix(mic: &[f32], room: &[f32]) -> Vec<f32> {
    let both = !mic.is_empty() && !room.is_empty();
    let gain = if both { 0.7 } else { 1.0 };
    let n = mic.len().max(room.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let sample = mic.get(i).copied().unwrap_or(0.0) * gain + room.get(i).copied().unwrap_or(0.0) * gain;
        out.push(sample.clamp(-1.0, 1.0));
    }
    out
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |max, sample| max.max(sample.abs()))
}

fn write_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
        for sample in samples {
            let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(value).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
    }
    Ok(cursor.into_inner())
}

async fn transcribe(wav: Vec<u8>, key: &str) -> Result<String, String> {
    if wav.len() > 24 * 1024 * 1024 {
        return Err("That meeting is too long for one note. I keep about ten minutes.".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("meeting.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", part);
    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("I couldn't send the recording ({e})."))?;
    let status = response.status();
    if !status.is_success() {
        let why = match status.as_u16() {
            401 | 403 => "Check the OpenAI key in AI Setup.",
            413 => "The recording was too big. I keep about ten minutes.",
            429 => "OpenAI is busy. Wait a moment and try again.",
            _ => "Check the OpenAI key, the model access, and your credits.",
        };
        return Err(format!("I couldn't turn the meeting into words (HTTP {}). {why}", status.as_u16()));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "OpenAI sent back something I didn't understand.".to_string())?;
    Ok(body["text"].as_str().unwrap_or("").trim().to_string())
}

fn summary_prompt(transcript: &str) -> String {
    let max = 12_000;
    let (body, extra) = if transcript.len() > max {
        let cut = transcript.floor_char_boundary(max);
        (&transcript[..cut], "\nThe rest was left out so the note stays short.\n")
    } else {
        (transcript, "")
    };
    format!(
        "Write meeting notes from this transcript. Use exactly these headings:\nDecisions:\nAction items:\nRecap:\n\nUnder Decisions and Action items, use one \"- \" bullet per item.\nAction items should name a person when the transcript says who, otherwise say \"someone\".\nDo not invent anything that was not said.\nIf the transcript is mostly silence or unclear, say so in the recap.\n{extra}\nTranscript:\n{body}"
    )
}

fn clean_bullet(line: &str) -> String {
    let trimmed = line.trim().trim_start_matches(['-', '*', '•']).trim();
    if let Some((index, rest)) = trimmed.split_once('.') {
        if !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()) {
            return rest.trim().to_string();
        }
    }
    trimmed.to_string()
}

fn heading(line: &str) -> Option<&'static str> {
    let name = line
        .trim()
        .trim_start_matches('#')
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_lowercase();
    match name.as_str() {
        "decisions" | "decision" => Some("decisions"),
        "action items" | "actions" | "action item" => Some("actions"),
        "recap" | "summary" => Some("recap"),
        _ => None,
    }
}

fn parse_summary(text: &str) -> (Vec<String>, Vec<String>, String) {
    let mut section = "recap";
    let mut decisions = Vec::new();
    let mut actions = Vec::new();
    let mut recap = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(next) = heading(trimmed) {
            section = next;
            continue;
        }
        let bullet = clean_bullet(trimmed);
        if bullet.is_empty() {
            continue;
        }
        match section {
            "decisions" => decisions.push(bullet),
            "actions" => actions.push(bullet),
            _ => recap.push(trimmed.to_string()),
        }
    }
    let recap = if decisions.is_empty() && actions.is_empty() && recap.is_empty() {
        text.trim().to_string()
    } else {
        recap.join("\n")
    };
    (decisions, actions, recap)
}

fn remember(app: &AppHandle, note: MeetingNote) -> Result<(), String> {
    let mut notes = match load_notes(app) {
        Ok(notes) => notes,
        Err(_) => {
            let path = notes_path(app)?;
            if path.exists() {
                let _ = std::fs::rename(&path, path.with_extension("json.bak"));
            }
            Vec::new()
        }
    };
    notes.insert(0, note);
    notes.truncate(20);
    save_notes(app, &notes)
}

#[tauri::command]
pub async fn stop_meeting(app: AppHandle) -> Result<MeetingNote, String> {
    let session = lock_session().take().ok_or("I'm not listening right now.")?;
    session.stop.store(true, Ordering::Relaxed);
    for handle in session.handles {
        let _ = handle.join();
    }
    let mixed = mix(&take_samples(&session.mic), &take_samples(&session.room));
    if mixed.len() < SAMPLE_RATE as usize / 2 || peak(&mixed) < 0.004 {
        return Err("That was too quiet for notes. Sit a little closer, or make sure the meeting is playing on this PC.".into());
    }
    let trimmed = session.capped.load(Ordering::Relaxed);
    let wav = write_wav(&mixed)?;
    let key = openai_key()?;
    let transcript = transcribe(wav, &key).await?;
    if transcript.is_empty() {
        return Err("I couldn't hear any words. Try again a little closer to the microphone.".into());
    }
    let prompt = summary_prompt(&transcript);
    let summary = match crate::complete_prompt(&app, &prompt, 16_000).await {
        Ok(text) => text,
        Err(error) => format!("Recap:\nI heard the meeting, but the notes model was busy ({error}).\n\n{transcript}"),
    };
    let (decisions, actions, recap) = parse_summary(&summary);
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let note = MeetingNote {
        id: format!("{created}-{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_millis()).unwrap_or(0)),
        created,
        decisions,
        actions,
        recap,
        heard: session.heard,
        transcript,
        trimmed,
    };
    remember(&app, note.clone())?;
    Ok(note)
}

#[tauri::command]
pub fn meeting_listening() -> bool {
    lock_session().is_some()
}

#[tauri::command]
pub fn list_meetings(app: AppHandle) -> Result<Vec<MeetingNote>, String> {
    load_notes(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_note_headings() {
        let text = "Decisions:\n- Ship on Friday\n\nAction items:\n1. Ada will send the notes\n\nRecap:\nWe agreed to ship.";
        let (decisions, actions, recap) = parse_summary(text);
        assert_eq!(decisions, vec!["Ship on Friday".to_string()]);
        assert_eq!(actions, vec!["Ada will send the notes".to_string()]);
        assert_eq!(recap, "We agreed to ship.");
    }

    #[test]
    fn resampler_halves_a_short_tone() {
        let mut resampler = Resampler::new(32_000);
        let mut out = Vec::new();
        resampler.push(&[0.0, 1.0, 0.0, 1.0, 0.0, 1.0], &mut out);
        assert!(out.len() >= 2);
        assert!(out.iter().all(|sample| sample.is_finite()));
    }
}
