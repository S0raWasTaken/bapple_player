#![warn(clippy::pedantic)]

use std::{
    collections::VecDeque,
    error::Error,
    ffi::OsString,
    fs::File,
    io::{Cursor, Read, Write, stdout},
    path::PathBuf,
    process::exit,
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use clap::{Parser, crate_version};
use serde::Deserialize;
use tar::{Archive, Entry};
use zstd::decode_all;

/// Asciix on cocaine
#[derive(Parser, Debug)]
#[command(version(crate_version!()))]
struct Args {
    /// Path to a .bapple file.
    file: PathBuf,
    /// Should be self-explanatory.
    #[arg(default_value = "0", value_parser = validate_fps)]
    frames_per_second: f64,
    /// Enables looping
    #[arg(short, long)]
    r#loop: bool,
}

fn validate_fps(s: &str) -> std::result::Result<f64, String> {
    let fps: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if fps < 0.01 {
        return Err("FPS value is too small.".to_string());
    }
    Ok(fps)
}

#[non_exhaustive]
#[derive(Deserialize, Default)]
struct Metadata {
    frametime: u64,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// I don't even care about the static mut here
// Only one thread is writing and the other one is reading
static mut SYNC_COUNTER: usize = 0;

fn main() -> Result<()> {
    let args = Args::parse();

    // I'll shove every single frame in here, and then sync with the
    // outside buffer every 15 frames.
    let mut internal_buffer = Vec::new();

    // Let's load this bad boy up first
    let (mp3_buf, frametime) = load_frames(&mut internal_buffer, args.file)?;

    let length = internal_buffer.len();

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // Shush, clippy
    let frametime = if args.frames_per_second == 0.0 {
        frametime
    } else {
        (1_000_000.0 / args.frames_per_second).round() as u64
    };

    if frametime == 0 {
        eprintln!(
            "Couldn't automatically detect the framerate.\n\
             You might wanna pass a value or maybe recompile the .bapple file."
        );
        exit(1);
    }

    let frametime = Duration::from_micros(frametime);
    loop {
        play(frametime, &internal_buffer, mp3_buf.clone(), length)?;
        if !args.r#loop {
            break;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn check_alsa_config() {
    use std::path::Path;

    let config_path = Path::new("/etc/alsa/conf.d");

    if !config_path.exists() {
        eprintln!("Warning: ALSA may not be configured for PipeWire/PulseAudio");
        eprintln!("If audio doesn't work, run:");
        eprintln!("  sudo mkdir -p /etc/alsa/conf.d");
        eprintln!(
            "  echo 'pcm.!default {{ type pipewire }}' | sudo tee /etc/alsa/conf.d/99-pipewire.conf"
        );
        eprintln!(
            "  echo 'ctl.!default {{ type pipewire }}' | sudo tee -a /etc/alsa/conf.d/99-pipewire.conf\n"
        );
        sleep(Duration::from_secs(5));
    }
}

fn play(
    frametime: Duration,
    internal_buffer: &[Vec<u8>],
    mp3_buf: Vec<u8>,
    length: usize,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    check_alsa_config(); // no-op on Windows, but who the hell is running this on Windows anyway? Every single terminal emulator sucks there.

    // Rodio spawns a new thread by itself
    let stream_handler = rodio::OutputStreamBuilder::open_default_stream()?;

    let sink = rodio::play(stream_handler.mixer(), Cursor::new(mp3_buf))?;

    spawn(move || outside_counter(frametime, length));

    let mut lock = stdout().lock();
    let mut internal_counter = 0;

    while internal_counter < length {
        let task_time = Instant::now();

        let decompressed_frame = decode_all(&*internal_buffer[internal_counter])?;

        lock.write_all(b"\r\x1b[2J\r\x1b[H")?;
        lock.write_all(&decompressed_frame)?;

        if internal_counter % 15 == 0 {
            resync(&mut internal_counter);
        } else {
            internal_counter += 1;
        }

        let elapsed = task_time.elapsed();
        if elapsed < frametime {
            sleep(frametime - elapsed);
        }
    }
    sink.stop();
    Ok(())
}

fn outside_counter(frametime: Duration, length: usize) {
    let mut counter = 0;
    while counter < length {
        sleep(frametime);
        counter += 1;
        unsafe { SYNC_COUNTER = counter }
    }
}

fn resync(internal_counter: &mut usize) {
    unsafe {
        *internal_counter = SYNC_COUNTER;
    }
}

fn load_frames(buf: &mut Vec<Vec<u8>>, path: PathBuf) -> Result<(Vec<u8>, u64)> {
    println!("Loading...\n");
    let tar_file = File::open(path)?;
    let mut archive = Archive::new(tar_file);

    let mut files = archive
        .entries()?
        .map(|e| -> Result<(usize, Vec<u8>)> {
            let mut e = e?;
            let file_stem = get_file_stem(&e)?;

            let mut content = Vec::new();
            e.read_to_end(&mut content)?;

            if file_stem == *"audio" {
                return Ok((0, content));
            }
            if file_stem == *"metadata" {
                return Ok((usize::MAX, content));
            }
            let file_number = file_stem
                .to_str()
                .ok_or("Frame filename is not valid UTF-8")?
                .parse::<usize>()?;
            Ok((file_number, content))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    drop(archive);

    files.sort_by_key(|e| e.0);
    let mut files = files.iter().map(|(_, b)| b).collect::<VecDeque<_>>();

    let audio_file = files.pop_front().unwrap();
    let frametime = files
        .pop_back()
        .and_then(|m| {
            let Metadata { frametime } = ron::de::from_bytes(m).unwrap_or_default();
            if frametime == 0 {
                None
            } else {
                Some(frametime)
            }
        })
        .unwrap_or_default();

    while let Some(compressed_frame) = files.pop_front() {
        buf.push(compressed_frame.as_slice().to_vec());
    }

    Ok((audio_file.clone(), frametime))
}

const FILE_STEM_ERR: &str = "
A frame file is missing its stem.
Is the .bapple archive corrupted?";
#[inline]
fn get_file_stem(e: &'_ Entry<File>) -> Result<OsString> {
    Ok(e.header()
        .path()?
        .file_stem()
        .ok_or(FILE_STEM_ERR)?
        .to_os_string())
}
