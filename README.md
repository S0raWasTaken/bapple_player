# .bapple player
This binary crate was made solely because [asciix](https://github.com/S0raWasTaken/bad_apple/tree/master/asciix) tends to desync a lot with the audio, and since I don't wanna mess with its current dynamic decompression workflow, I'm making a whole new player, **which is not meant to work in extremely cpu and ram-limited environments like its predecessor**.

> Matter of fact, this is gonna load every single frame into your RAM. <br/>
> Good luck if you plan on using this <3

### Installation
If you're masochistic enough or if your settings are buffed enough for installing this, then you came to the right place!

The installation method is still the same from the original [bad_apple](https://github.com/s0rawastaken/bad_apple) crate.

```sh
git clone https://github.com/S0raWasTaken/bapple_player --depth 1
cargo install --path bapple_player
```
This technically works in Windows, but I swear that every terminal emulator sucks there. There's not as many stutters and flashing stuff on Linux (assuming you're using a GPU-accelerated terminal like [kitty](https://github.com/kovidgoyal/kitty)).

If you find any way to make this not perform horribly in Windows, go find my email or open an issue, I'd love to know!