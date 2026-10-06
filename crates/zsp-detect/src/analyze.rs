//! Analysis inputs: a small grayscale grid and per-cell edge strength.

/// Width of every analysis grid, in cells.
pub const GRID_W: usize = 160;

/// Grayscale analysis image, row-major, `width * height` cells.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

/// Per-cell edge strength: the mean absolute 4-neighbour Laplacian of the full-size
/// luma image inside each cell. Row-major, same shape as the matching [`Grid`].
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

/// Grid size `(width, height)` for a capture region of `width` x `height` pixels:
/// always [`GRID_W`] wide; height follows the aspect ratio
/// (`round(GRID_W * height / width)`, round half to even as numpy does), clamped to 30..=240.
///
/// Reference: `grid_shape` in samples/proto/detect.py.
pub fn grid_shape(width: u32, height: u32) -> (usize, usize) {
    let _ = (width, height);
    todo!("port grid_shape from samples/proto/detect.py")
}

/// Build the analysis inputs from an RGBA8 frame (row-major, 4 bytes per pixel, alpha ignored).
///
/// - Luma: Pillow's "L" conversion, `(R*19595 + G*38470 + B*7471 + 0x8000) >> 16`.
/// - Grid: downscale the luma image to [`grid_shape`] with a separable triangle filter that
///   matches Pillow's `resize(..., BILINEAR)` (support scaled by the reduction factor,
///   weights normalised per output pixel, rounded to u8). ACCEPTANCE B1 allows a mean
///   difference of 1.0 and a maximum of 8 against the reference.
/// - Detail: Laplacian `|4c - up - down - left - right|` on the full-size luma as f32
///   (0 on the one-pixel border), averaged over each cell. Cell boundaries are
///   `floor(i * H / grid_h)` and `floor(j * W / GRID_W)`, as `np.linspace(...).astype(int)`.
///
/// Reference: `to_grid` and `to_detail` in samples/proto/detect.py.
pub fn analyze(rgba: &[u8], width: u32, height: u32) -> (Grid, Detail) {
    let _ = (rgba, width, height);
    todo!("port to_grid and to_detail from samples/proto/detect.py")
}
