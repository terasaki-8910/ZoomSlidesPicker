//! Frames of a video file, through the `ffmpeg` and `ffprobe` commands on PATH
//! (SPEC.md > 構成: not bundled). Used by the mp4 mode and by tests.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

use image::RgbaImage;

/// True when both `ffmpeg` and `ffprobe` run (`-version` exits successfully).
pub fn ffmpeg_available() -> bool {
    runs_version("ffmpeg") && runs_version("ffprobe")
}

fn runs_version(program: &str) -> bool {
    Command::new(program)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// `ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0
/// <file>`, which prints a single `<width>x<height>` line.
fn probe_size(path: &Path) -> io::Result<(u32, u32)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.trim();
    let (w, h) = line.split_once('x').ok_or_else(|| {
        io::Error::other(format!(
            "unexpected ffprobe output for {}: {line:?}",
            path.display()
        ))
    })?;
    let width: u32 = w
        .trim()
        .parse()
        .map_err(|_| io::Error::other(format!("bad width in ffprobe output: {line:?}")))?;
    let height: u32 = h
        .trim()
        .parse()
        .map_err(|_| io::Error::other(format!("bad height in ffprobe output: {line:?}")))?;
    Ok((width, height))
}

/// Decoded frames at a fixed rate: frame `i` is at time `i / fps` seconds.
/// Runs `ffmpeg -v error -i <file> -vf fps=<fps> -f rawvideo -pix_fmt rgba -` and reads
/// width x height x 4 bytes per frame; the size comes from `ffprobe`.
pub struct Mp4Frames {
    width: u32,
    height: u32,
    fps: f64,
    frame: u64,
    child: Child,
    stdout: ChildStdout,
}

impl Mp4Frames {
    pub fn open(path: &Path, fps: f64) -> io::Result<Self> {
        let (width, height) = probe_size(path)?;

        let mut child = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args([
                "-vf",
                &format!("fps={fps}"),
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .expect("ffmpeg was spawned with a piped stdout");

        Ok(Mp4Frames {
            width,
            height,
            fps,
            frame: 0,
            child,
            stdout,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Iterator for Mp4Frames {
    type Item = io::Result<(f64, RgbaImage)>;

    fn next(&mut self) -> Option<Self::Item> {
        let frame_bytes = self.width as usize * self.height as usize * 4;
        let mut buf = vec![0u8; frame_bytes];
        match self.stdout.read_exact(&mut buf) {
            Ok(()) => {
                let t = self.frame as f64 / self.fps;
                self.frame += 1;
                let img = RgbaImage::from_raw(self.width, self.height, buf)
                    .expect("buf has exactly width * height * 4 bytes");
                Some(Ok((t, img)))
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl Drop for Mp4Frames {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
