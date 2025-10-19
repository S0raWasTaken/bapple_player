# .bapple player
This binary crate was made solely because [asciix](https://github.com/S0raWasTaken/bad_apple/tree/master/asciix) tends to desync a lot with the audio, and it was
so cluttered that I decided to rewrite it entirely.

It now consumes about 30% less RAM than asciix, and no matter how bad the video
is running, it never desyncs.

### Installation

The installation method is still the same from the original [bad_apple](https://github.com/s0rawastaken/bad_apple) crate.

```sh
git clone https://github.com/S0raWasTaken/bapple_player --depth 1
cargo install --path bapple_player
```
This technically works in Windows, but I swear that every terminal emulator sucks there. There's not as many stutters and flashing stuff on Linux (assuming you're using a GPU-accelerated terminal like [kitty](https://github.com/kovidgoyal/kitty)).

If you find any way to make this not perform horribly in Windows, go find my email or open an issue, I'd love to know!

### Usage
Check [the asciic instructions](https://github.com/S0raWasTaken/bad_apple/tree/master/asciic) to learn how to create your own .bapple ascii video files.

Usually the framerate shows up in the ffmpeg output during the video extraction process, so that's what we're gonna use.

```sh
λ bplay --help
Asciix on cocaine

Usage: bplay [OPTIONS] <FILE> <FRAMES_PER_SECOND>

Arguments:
  <FILE>               Path to a .bapple file
  <FRAMES_PER_SECOND>  Should be self-explanatory

Options:
  -l, --loop     Enables looping
  -h, --help     Print help
  -V, --version  Print version
```
#### Examples:
```sh
bplay video.bapple 30
```

```sh
bplay gif.bapple 24 --loop
```