use std::{
    collections::VecDeque,
    error::Error,
    ffi::OsString,
    fs::{File, write},
    io::{Read, Write, stdout},
    path::PathBuf,
    process::{Command, exit},
    sync::mpsc::{self, Receiver},
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use clap::{Parser, crate_version};
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
    frames_per_second: u64,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// I don't even care about the static mut here
// Only one thread is writing and the other one is reading
static mut SYNC_COUNTER: usize = 0;

fn main() -> Result<()> {
    let args = Args::parse();
    let path = args.file;
    let fps = args.frames_per_second;
    let delay_micros = Duration::from_micros(1_000_000 / fps);

    // I'll shove every single frame in here, and then sync with the
    // outside buffer every 15 frames.
    let mut internal_buffer = Vec::new();

    // Let's load this bad boy up first          ↓↓↓↓↓↓↓↓↓↓↓↓↓↓
    let mp3_buf = load_frames(&mut internal_buffer, path)?;

    let length = internal_buffer.len();

    let (tx1, rx1) = mpsc::channel::<()>(); // Just blocking logic
    let (tx2, rx2) = mpsc::channel::<()>();

    spawn(|| play_audio(mp3_buf, rx1));
    spawn(move || outside_counter(delay_micros, length, rx2));

    let mut lock = stdout().lock();

    let mut internal_counter = 0;

    tx1.send(()).ok();
    tx2.send(()).ok();
    while internal_counter < length {
        let task_time = Instant::now();

        lock.write_all(b"\r\x1b[2J\r\x1b[H")?;
        lock.write_all(&internal_buffer[internal_counter])?;

        if internal_counter % 15 == 0 {
            resync(&mut internal_counter);
        } else {
            internal_counter += 1;
        }

        let elapsed = task_time.elapsed();
        if elapsed < delay_micros {
            sleep(delay_micros - elapsed);
        }
    }

    Ok(())
}

fn outside_counter(delay_micros: Duration, length: usize, start: Receiver<()>) {
    start.recv().ok();
    let mut counter = 0;
    while counter < length {
        sleep(delay_micros);
        counter += 1;
        unsafe { SYNC_COUNTER = counter }
    }
}

fn resync(internal_counter: &mut usize) {
    unsafe {
        *internal_counter = SYNC_COUNTER;
    }
}

/// Decompresses every single one of the frames and shoves it into the buffer
/// -> Returns the audio file
fn load_frames(buf: &mut Vec<Vec<u8>>, path: PathBuf) -> Result<Vec<u8>> {
    println!("Loading...");
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
            let file_number = closure_error!(file_stem.to_str().unwrap().parse::<usize>());

            (file_number, content)
        })
        .collect::<Vec<_>>();

    drop(archive);

    files.sort_by_key(|e| e.0);
    let mut files = files.iter().map(|(_, b)| b).collect::<VecDeque<_>>();

    let audio_file = files.pop_front().unwrap();

    // It's show time
    while let Some(compressed_frame) = files.pop_front() {
        buf.push(decode_all(compressed_frame.as_slice())?);
    }

    Ok(audio_file.clone())
}

// borrowed stuff from asciix

fn play_audio(mp3_buf: Vec<u8>, start: Receiver<()>) {
    let Ok(tmp_dir) = TempDir::new() else {
        return;
    };
    let mut file_path = tmp_dir.path().to_path_buf();
    file_path.set_file_name("audio");
    file_path.set_extension("mp3");

    if write(&file_path, mp3_buf).is_err() {
        return;
    }

    start.recv().ok();
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
