#[cfg(target_os = "linux")]
pub const ALSA_WARNING: &str = "
\x1b[33m[warning]\x1b[0m ALSA may not be configured for your audio server.
If you use pipewire AND the audio doesn't work, try running:
    echo 'pcm.!default { type pipewire }' | sudo tee /etc/alsa/conf.d/99-pipewire.conf
    echo 'ctl.!default { type pipewire }' | sudo tee -a /etc/alsa/conf.d/99-pipewire.conf
";

pub const FRAMETIME_ZERO: &str = "
\x1b[31m[fatal]\x1b[0m The .bapple file you tried to play was likely compiled with an old version of asciic, or it's corrupted.
The file metadata could not be parsed.
Please, try re-converting the file or passing an FPS value as such:
    bplay <fps>
";
