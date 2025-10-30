#![warn(clippy::pedantic)]

use std::{
    collections::VecDeque,
    error::Error,
    ffi::OsString,
    fs::{File, write},
    io::{Read, Write, stdout},
    path::PathBuf,
    process::{Command, exit},
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use clap::{Parser, crate_version};
use serde::Deserialize;
use tar::{Archive, Entry};
use tempfile::TempDir;
use zstd::decode_all;

/// Asciix on cocaine
#[derive(Parser, Debug)]
#[command(version(crate_version!()))]
struct Args {
    /// Path to a .bapple file.
    file: PathBuf,
    /// Should be self-explanatory.
    #[arg(default_value = "0")]
    frames_per_second: u64,
    /// Enables looping
    #[arg(short, long)]
    r#loop: bool,
}

#[non_exhaustive]
#[derive(Deserialize, Default)]
struct Metadata {
    fps: u64,
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
    let (mp3_buf, fps) = load_frames(&mut internal_buffer, args.file)?;

    let length = internal_buffer.len();

    let fps = if args.frames_per_second == 0 {
        fps
    } else {
        args.frames_per_second
    };

    if fps == 0 {
        eprintln!(
            "Couldn't automatically detect the framerate.\n\
             You might wanna pass a value or maybe recompile the .bapple file."
        );
        exit(1);
    }

    let frametime = Duration::from_micros(1_000_000 / fps);
    loop {
        play(frametime, &internal_buffer, mp3_buf.clone(),length)?;
        if !args.r#loop {
            break;
        }
    }
    Ok(())
}

fn play(
    frametime: Duration,
    internal_buffer: &[Vec<u8>],
    mp3_buf: Vec<u8>,
    length: usize,
) -> Result<()> {
    spawn(move || play_audio(&mp3_buf));
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
        .map(|e| closure_error!(e))
        .map(|mut e| {
            let file_stem = get_file_stem(&e).unwrap();

            let mut content = Vec::new();
            closure_error!(e.read_to_end(&mut content));

            if file_stem == *"audio" {
                return (0, content);
            }
            if file_stem == *"metadata" {
                return (usize::MAX, content);
            }
            let file_number = closure_error!(file_stem.to_str().unwrap().parse::<usize>());

            (file_number, content)
        })
        .collect::<Vec<_>>();

    drop(archive);

    files.sort_by_key(|e| e.0);
    let mut files = files.iter().map(|(_, b)| b).collect::<VecDeque<_>>();

    let audio_file = files.pop_front().unwrap();
    let fps = files
        .pop_back()
        .map(|m| {
            let Metadata { fps } = ron::de::from_bytes(m).unwrap_or_default();
            fps
        })
        .unwrap();

    while let Some(compressed_frame) = files.pop_front() {
        buf.push(compressed_frame.as_slice().to_vec());
    }

    Ok((audio_file.clone(), fps))
}

// borrowed stuff from asciix

fn play_audio(mp3_buf: &[u8]) {
    let Ok(tmp_dir) = TempDir::new() else {
        return;
    };
    let mut file_path = tmp_dir.path().to_path_buf();
    file_path.set_file_name("audio");
    file_path.set_extension("mp3");

    if write(&file_path, mp3_buf).is_err() {
        return;
    }

    Command::new("mpv").args([file_path]).output().ok();
}

#[inline]
fn get_file_stem(e: &'_ Entry<File>) -> Option<OsString> {
    Some(e.header().path().ok()?.file_stem()?.to_os_string())
}

#[macro_export]
macro_rules! closure_error {
    ($x:expr) => {
        match $x {
            Ok(res) => res,
            Err(e) => {
                eprintln!("{e:#?}");
                exit(7);
            }
        }
    };
}
