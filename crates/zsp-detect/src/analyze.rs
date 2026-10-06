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
    let raw = (GRID_W as f64) * (height as f64) / (width as f64);
    let gh = raw.round_ties_even().clamp(30.0, 240.0) as usize;
    (GRID_W, gh)
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
    let in_w = width as usize;
    let in_h = height as usize;
    let (gw, gh) = grid_shape(width, height);
    let luma = to_luma(rgba, in_w, in_h);
    let grid = to_grid(&luma, in_w, in_h, gw, gh);
    let detail = to_detail(&luma, in_w, in_h, gw, gh);
    (
        Grid {
            width: gw,
            height: gh,
            data: grid,
        },
        Detail {
            width: gw,
            height: gh,
            data: detail,
        },
    )
}

/// Pillow "L" conversion, alpha ignored.
fn to_luma(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; width * height];
    for (px, o) in rgba.chunks_exact(4).zip(out.iter_mut()) {
        let (r, g, b) = (px[0] as u32, px[1] as u32, px[2] as u32);
        *o = ((r * 19595 + g * 38470 + b * 7471 + 0x8000) >> 16) as u8;
    }
    out
}

/// Round to the nearest integer (ties away from zero, as Pillow's `clip8` does after its
/// fixed-point sum) and clamp to the `u8` range.
fn round_clamp_u8(x: f32) -> u8 {
    x.round().clamp(0.0, 255.0) as u8
}

/// Per-output-index taps for a separable triangle-filter resize from `in_len` to `out_len`,
/// matching Pillow's `resize(..., BILINEAR)` when downscaling: `(first_input_index,
/// normalised_weights)` for each output index.
fn resize_taps(in_len: usize, out_len: usize) -> Vec<(usize, Vec<f32>)> {
    let in_f = in_len as f64;
    let out_f = out_len as f64;
    let scale = in_f / out_f;
    let support = scale;
    (0..out_len)
        .map(|i| {
            let center = (i as f64 + 0.5) * scale;
            let xmin = (center - support + 0.5).floor().max(0.0) as usize;
            let xmax = (center + support + 0.5).floor().min(in_f) as usize;
            let mut weights: Vec<f32> = (xmin..xmax)
                .map(|x| (1.0 - ((x as f64 - center + 0.5) / scale).abs()).max(0.0) as f32)
                .collect();
            let sum: f32 = weights.iter().sum();
            if sum > 0.0 {
                for w in &mut weights {
                    *w /= sum;
                }
            }
            (xmin, weights)
        })
        .collect()
}

/// Horizontal pass: resize each of `h` rows of width `in_w` to width `taps.len()`.
fn resize_horizontal(src: &[u8], in_w: usize, h: usize, taps: &[(usize, Vec<f32>)]) -> Vec<u8> {
    let out_w = taps.len();
    let mut out = vec![0u8; out_w * h];
    for y in 0..h {
        let row = &src[y * in_w..(y + 1) * in_w];
        let out_row = &mut out[y * out_w..(y + 1) * out_w];
        for (i, (xmin, weights)) in taps.iter().enumerate() {
            let mut acc = 0.0f32;
            for (k, &w) in weights.iter().enumerate() {
                acc += row[xmin + k] as f32 * w;
            }
            out_row[i] = round_clamp_u8(acc);
        }
    }
    out
}

/// Vertical pass: resize `taps`-many output rows from the `w`-wide input, combining whole
/// input rows with the precomputed weights.
fn resize_vertical(src: &[u8], w: usize, taps: &[(usize, Vec<f32>)]) -> Vec<u8> {
    let out_h = taps.len();
    let mut out = vec![0u8; w * out_h];
    let mut acc = vec![0.0f32; w];
    for (j, (ymin, weights)) in taps.iter().enumerate() {
        acc.iter_mut().for_each(|v| *v = 0.0);
        for (k, &wt) in weights.iter().enumerate() {
            let row = &src[(ymin + k) * w..(ymin + k + 1) * w];
            for (a, &v) in acc.iter_mut().zip(row) {
                *a += v as f32 * wt;
            }
        }
        let out_row = &mut out[j * w..(j + 1) * w];
        for (o, &a) in out_row.iter_mut().zip(&acc) {
            *o = round_clamp_u8(a);
        }
    }
    out
}

/// Pillow-equivalent `BILINEAR` downscale: horizontal pass first, then vertical.
fn to_grid(luma: &[u8], in_w: usize, in_h: usize, out_w: usize, out_h: usize) -> Vec<u8> {
    let h_taps = resize_taps(in_w, out_w);
    let stage1 = resize_horizontal(luma, in_w, in_h, &h_taps);
    let v_taps = resize_taps(in_h, out_h);
    resize_vertical(&stage1, out_w, &v_taps)
}

/// `floor(i * len / parts)` for `i` in `0..=parts`, computed in `f64` (matches
/// `np.linspace(0, len, parts + 1).astype(int)`).
fn cell_bounds(len: usize, parts: usize) -> Vec<usize> {
    (0..=parts)
        .map(|i| ((i as f64) * (len as f64) / (parts as f64)).floor() as usize)
        .collect()
}

/// The cell index `c` for each position in `0..len`, where `bounds[c] <= position <
/// bounds[c + 1]` (`bounds` has `parts + 1` entries, so `c` stays within `0..parts`).
fn cell_index(bounds: &[usize], len: usize) -> Vec<usize> {
    let mut out = vec![0usize; len];
    let mut c = 0;
    for (i, slot) in out.iter_mut().enumerate() {
        while c + 1 < bounds.len() - 1 && bounds[c + 1] <= i {
            c += 1;
        }
        *slot = c;
    }
    out
}

/// Mean absolute 4-neighbour Laplacian of `luma`, per cell of the `out_w` x `out_h` grid.
/// The Laplacian is 0 on the one-pixel border, same as the reference.
fn to_detail(luma: &[u8], in_w: usize, in_h: usize, out_w: usize, out_h: usize) -> Vec<f32> {
    let row_bounds = cell_bounds(in_h, out_h);
    let col_bounds = cell_bounds(in_w, out_w);
    let row_cell = cell_index(&row_bounds, in_h);
    let col_cell = cell_index(&col_bounds, in_w);

    let mut sums = vec![0.0f32; out_w * out_h];
    for (y, &cy) in row_cell
        .iter()
        .enumerate()
        .take(in_h.saturating_sub(1))
        .skip(1)
    {
        let base = y * in_w;
        let sums_row = &mut sums[cy * out_w..(cy + 1) * out_w];
        for x in 1..in_w.saturating_sub(1) {
            let c = luma[base + x] as f32;
            let lap = 4.0 * c
                - luma[base - in_w + x] as f32
                - luma[base + in_w + x] as f32
                - luma[base + x - 1] as f32
                - luma[base + x + 1] as f32;
            sums_row[col_cell[x]] += lap.abs();
        }
    }

    let mut out = vec![0.0f32; out_w * out_h];
    for cy in 0..out_h {
        let rows_n = (row_bounds[cy + 1] - row_bounds[cy]) as f32;
        for cx in 0..out_w {
            let cols_n = (col_bounds[cx + 1] - col_bounds[cx]) as f32;
            let area = rows_n * cols_n;
            if area > 0.0 {
                out[cy * out_w + cx] = sums[cy * out_w + cx] / area;
            }
        }
    }
    out
}
