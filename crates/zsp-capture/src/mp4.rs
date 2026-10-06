//! Frames of a video file, through the `ffmpeg` and `ffprobe` commands on PATH
//! (SPEC.md > 構成: not bundled). Used by the mp4 mode and by tests.

use std::io;
use std::path::Path;

use image::RgbaImage;

/// True when both `ffmpeg` and `ffprobe` run (`-version` exits successfully).
pub fn ffmpeg_available() -> bool {
    todo!()
}

/// Decoded frames at a fixed rate: frame `i` is at time `i / fps` seconds.
/// Runs `ffmpeg -v error -i <file> -vf fps=<fps> -f rawvideo -pix_fmt rgba -` and reads
/// width x height x 4 bytes per frame; the size comes from `ffprobe`.
pub struct Mp4Frames {
    width: u32,
    height: u32,
}

impl Mp4Frames {
    pub fn open(path: &Path, fps: f64) -> io::Result<Self> {
        let _ = (path, fps);
        todo!()
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Iterator for Mp4Frames {
    type Item = io::Result<(f64, RgbaImage)>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
