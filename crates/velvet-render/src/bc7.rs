//! Texture compression: RGBA8 images to BC7 (mode 6) blocks, a quarter of the memory.
//!
//! Mode 6 keeps one pair of RGBA endpoints a 4×4 block (7 bits a channel and a shared low bit per
//! endpoint) and a 4-bit index a texel: sixteen shades along the line between them. The encoder
//! finds that line along the block's principal axis (power iteration on its covariance), snaps
//! the ends to what the format can hold, picks each texel's shade, then refits the ends to the
//! shades chosen (least squares) and picks again. Blocks are shared among the cores.

const WEIGHTS: [u32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

fn interpolate(e0: [u32; 4], e1: [u32; 4], w: u32) -> [u32; 4] {
    [0, 1, 2, 3].map(|c| ((64 - w) * e0[c] + w * e1[c] + 32) >> 6)
}

/// The best 7-bit-plus-shared-bit endpoint for an RGBA value (0..255 floats).
fn quantize(v: [f32; 4]) -> ([u32; 4], u32) {
    let mut best = ([0u32; 4], 0u32, f32::INFINITY);
    for p in 0..2u32 {
        let mut c = [0u32; 4];
        let mut err = 0.0;
        for k in 0..4 {
            let q = ((v[k] - p as f32) / 2.0).round().clamp(0.0, 127.0) as u32;
            c[k] = q;
            let e = (q * 2 + p) as f32 - v[k];
            err += e * e;
        }
        if err < best.2 {
            best = (c, p, err);
        }
    }
    (best.0, best.1)
}

fn expand(c: [u32; 4], p: u32) -> [u32; 4] {
    c.map(|v| v * 2 + p)
}

/// Each texel's shade for these endpoints, and the total squared error.
fn pick(texels: &[[u8; 4]; 16], e0: [u32; 4], e1: [u32; 4]) -> ([u8; 16], u32) {
    let palette: [[u32; 4]; 16] = std::array::from_fn(|i| interpolate(e0, e1, WEIGHTS[i]));
    let mut indices = [0u8; 16];
    let mut total = 0u32;
    for (t, texel) in texels.iter().enumerate() {
        let mut best = (0u8, u32::MAX);
        for (i, c) in palette.iter().enumerate() {
            let mut d = 0u32;
            for k in 0..4 {
                let e = c[k] as i32 - texel[k] as i32;
                d += (e * e) as u32;
            }
            if d < best.1 {
                best = (i as u8, d);
            }
        }
        indices[t] = best.0;
        total += best.1;
    }
    (indices, total)
}

/// The block's principal axis (RGBA) and its mean.
fn principal(texels: &[[u8; 4]; 16]) -> ([f32; 4], [f32; 4]) {
    let mut mean = [0.0f32; 4];
    for t in texels {
        for k in 0..4 {
            mean[k] += t[k] as f32 / 16.0;
        }
    }
    let mut cov = [[0.0f32; 4]; 4];
    for t in texels {
        let d = [0, 1, 2, 3].map(|k| t[k] as f32 - mean[k]);
        for a in 0..4 {
            for b in 0..4 {
                cov[a][b] += d[a] * d[b];
            }
        }
    }
    let mut axis = [1.0f32, 1.0, 1.0, 0.25];
    for _ in 0..8 {
        let next = [0, 1, 2, 3].map(|a| (0..4).map(|b| cov[a][b] * axis[b]).sum::<f32>());
        let len = next.iter().map(|v| v * v).sum::<f32>().sqrt();
        if len < 1e-6 {
            break;
        }
        axis = next.map(|v| v / len);
    }
    (axis, mean)
}

/// Endpoints refitted to the chosen shades (least squares on the weights).
fn refit(texels: &[[u8; 4]; 16], indices: &[u8; 16]) -> Option<([f32; 4], [f32; 4])> {
    let (mut aa, mut ab, mut bb) = (0.0f32, 0.0f32, 0.0f32);
    let mut ax = [0.0f32; 4];
    let mut bx = [0.0f32; 4];
    for (t, &i) in texels.iter().zip(indices) {
        let w = WEIGHTS[i as usize] as f32 / 64.0;
        let (a, b) = (1.0 - w, w);
        aa += a * a;
        ab += a * b;
        bb += b * b;
        for k in 0..4 {
            ax[k] += a * t[k] as f32;
            bx[k] += b * t[k] as f32;
        }
    }
    let det = aa * bb - ab * ab;
    if det.abs() < 1e-4 {
        return None;
    }
    let e0 = [0, 1, 2, 3].map(|k| ((bb * ax[k] - ab * bx[k]) / det).clamp(0.0, 255.0));
    let e1 = [0, 1, 2, 3].map(|k| ((aa * bx[k] - ab * ax[k]) / det).clamp(0.0, 255.0));
    Some((e0, e1))
}

/// One 4×4 block of RGBA texels as a 16-byte BC7 mode 6 block.
#[cfg(test)]
pub(crate) fn encode_block(texels: &[[u8; 4]; 16]) -> [u8; 16] {
    encode_block_err(texels).0
}

/// The block and its total squared error (over the 64 channel values).
fn encode_block_err(texels: &[[u8; 4]; 16]) -> ([u8; 16], u32) {
    let (axis, mean) = principal(texels);
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for t in texels {
        let p: f32 = (0..4).map(|k| (t[k] as f32 - mean[k]) * axis[k]).sum();
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let end = |s: f32| [0, 1, 2, 3].map(|k| (mean[k] + axis[k] * s).clamp(0.0, 255.0));
    let (mut q0, mut p0) = quantize(end(lo));
    let (mut q1, mut p1) = quantize(end(hi));
    let (mut indices, mut err) = pick(texels, expand(q0, p0), expand(q1, p1));
    if err > 0 {
        if let Some((f0, f1)) = refit(texels, &indices) {
            let (r0, rp0) = quantize(f0);
            let (r1, rp1) = quantize(f1);
            let (ri, re) = pick(texels, expand(r0, rp0), expand(r1, rp1));
            if re < err {
                (q0, p0, q1, p1, indices, err) = (r0, rp0, r1, rp1, ri, re);
            }
        }
    }
    // The first texel's index must have its top bit clear: swap the ends if not.
    if indices[0] >= 8 {
        std::mem::swap(&mut q0, &mut q1);
        std::mem::swap(&mut p0, &mut p1);
        for i in indices.iter_mut() {
            *i = 15 - *i;
        }
    }
    // Pack, least significant bit first.
    let mut bits = 0u128;
    let mut at = 0u32;
    let mut put = |value: u32, count: u32| {
        bits |= ((value as u128) & ((1u128 << count) - 1)) << at;
        at += count;
    };
    put(1 << 6, 7);
    for k in 0..4 {
        put(q0[k], 7);
        put(q1[k], 7);
    }
    put(p0, 1);
    put(p1, 1);
    put(indices[0] as u32, 3);
    for &i in &indices[1..] {
        put(i as u32, 4);
    }
    (bits.to_le_bytes(), err)
}

/// A whole image (any size; the edges are padded by repeating the last texel) as BC7 blocks, row
/// of blocks after row of blocks.
#[cfg(test)]
pub(crate) fn encode(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    encode_measured(width, height, rgba).0
}

/// The blocks and how close they come to the image (PSNR in dB over every channel of every texel).
pub(crate) fn encode_measured(width: u32, height: u32, rgba: &[u8]) -> (Vec<u8>, f64) {
    let (bw, bh) = (width.div_ceil(4) as usize, height.div_ceil(4) as usize);
    let mut out = vec![0u8; bw * bh * 16];
    let error = std::sync::atomic::AtomicU64::new(0);
    let rows = |first: usize, chunk: &mut [u8]| {
        let mut sum = 0u64;
        for (r, row) in chunk.chunks_mut(bw * 16).enumerate() {
            let by = first + r;
            for bx in 0..bw {
                let texels: [[u8; 4]; 16] = std::array::from_fn(|t| {
                    let x = (bx * 4 + t % 4).min(width as usize - 1);
                    let y = (by * 4 + t / 4).min(height as usize - 1);
                    let k = (y * width as usize + x) * 4;
                    [rgba[k], rgba[k + 1], rgba[k + 2], rgba[k + 3]]
                });
                let (block, err) = encode_block_err(&texels);
                row[bx * 16..bx * 16 + 16].copy_from_slice(&block);
                sum += err as u64;
            }
        }
        error.fetch_add(sum, std::sync::atomic::Ordering::Relaxed);
    };
    let threads = crate::work::threads();
    if bh < 16 || threads < 2 {
        rows(0, &mut out);
    } else {
        // (On the kept threads: making new ones for every level cost memory never given back.)
        let per = bh.div_ceil(threads);
        crate::work::each(
            out.chunks_mut(per * bw * 16).enumerate().collect(),
            &|(k, chunk): (usize, &mut [u8])| rows(k * per, chunk),
        );
    }
    // (Padding texels count too; they repeat real ones, so they barely move it.)
    let mse = error.into_inner() as f64 / (bw * bh * 64) as f64;
    let psnr = if mse <= 0.0 {
        99.0
    } else {
        10.0 * (255.0f64 * 255.0 / mse).log10()
    };
    (out, psnr)
}

/// One BC7 mode 6 block back to texels (for tests).
#[cfg(test)]
pub(crate) fn decode_block(block: &[u8; 16]) -> [[u8; 4]; 16] {
    let bits = u128::from_le_bytes(*block);
    let mut at = 0u32;
    let mut get = |count: u32| {
        let v = ((bits >> at) & ((1u128 << count) - 1)) as u32;
        at += count;
        v
    };
    assert_eq!(get(7), 1 << 6, "mode 6");
    let mut q0 = [0u32; 4];
    let mut q1 = [0u32; 4];
    for k in 0..4 {
        q0[k] = get(7);
        q1[k] = get(7);
    }
    let (p0, p1) = (get(1), get(1));
    let (e0, e1) = (expand(q0, p0), expand(q1, p1));
    let mut indices = [0u32; 16];
    indices[0] = get(3);
    for i in indices.iter_mut().skip(1) {
        *i = get(4);
    }
    indices.map(|i| interpolate(e0, e1, WEIGHTS[i as usize]).map(|v| v as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn psnr(a: &[u8], b: &[u8]) -> f64 {
        let mse: f64 = a
            .iter()
            .zip(b)
            .map(|(x, y)| (*x as f64 - *y as f64).powi(2))
            .sum::<f64>()
            / a.len() as f64;
        if mse == 0.0 {
            99.0
        } else {
            10.0 * (255.0f64 * 255.0 / mse).log10()
        }
    }

    fn roundtrip(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let blocks = encode(width, height, rgba);
        let bw = width.div_ceil(4) as usize;
        let mut out = vec![0u8; rgba.len()];
        for by in 0..height.div_ceil(4) as usize {
            for bx in 0..bw {
                let k = (by * bw + bx) * 16;
                let texels = decode_block(blocks[k..k + 16].try_into().unwrap());
                for (t, texel) in texels.iter().enumerate() {
                    let (x, y) = (bx * 4 + t % 4, by * 4 + t / 4);
                    if x < width as usize && y < height as usize {
                        let o = (y * width as usize + x) * 4;
                        out[o..o + 4].copy_from_slice(texel);
                    }
                }
            }
        }
        out
    }

    #[test]
    fn flat_and_two_colour_blocks_come_back_exactly_or_nearly() {
        let flat = [[200u8, 120, 40, 255]; 16];
        let back = decode_block(&encode_block(&flat));
        for t in back {
            for k in 0..4 {
                assert!((t[k] as i32 - flat[0][k] as i32).abs() <= 1, "{t:?}");
            }
        }
        let two: [[u8; 4]; 16] = std::array::from_fn(|i| {
            if i % 2 == 0 {
                [10, 20, 30, 255]
            } else {
                [240, 230, 220, 0]
            }
        });
        let back = decode_block(&encode_block(&two));
        for (t, o) in back.iter().zip(&two) {
            for k in 0..4 {
                assert!((t[k] as i32 - o[k] as i32).abs() <= 2, "{t:?} vs {o:?}");
            }
        }
    }

    #[test]
    fn gradients_and_noise_keep_their_quality() {
        let (w, h) = (67u32, 45u32);
        let gradient: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let (x, y) = (i % w, i / w);
                [
                    (x * 255 / w) as u8,
                    (y * 255 / h) as u8,
                    ((x + y) * 2) as u8,
                    255,
                ]
            })
            .collect();
        let p = psnr(&gradient, &roundtrip(w, h, &gradient));
        assert!(p > 38.0, "gradient {p:.1} dB");
        let noise: Vec<u8> = (0..w * h * 4)
            .map(|k| (k.wrapping_mul(2654435761) >> 24) as u8)
            .collect();
        let p = psnr(&noise, &roundtrip(w, h, &noise));
        assert!(p > 12.0, "noise survives as noise: {p:.1} dB");
    }

    /// The game's own textures: `cargo test -p velvet-render --release bc7_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bc7_real() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../DemonQueen/juego/nucleo/data/assets/3d");
        for file in [
            "villa/villa_town.glb",
            "baths/baths_places.glb",
            "brothel/brothel_ground.glb",
            "baths/spa_girl.glb",
        ] {
            let Ok(model) = crate::GltfModel::load(root.join(file)) else {
                continue;
            };
            let (mut worst, mut sum, mut n, mut px) = (99.0f64, 0.0, 0, 0usize);
            let t0 = std::time::Instant::now();
            for t in model
                .textures
                .iter()
                .filter(|t| t.width >= 4 && t.height >= 4)
            {
                let p = psnr(&t.rgba, &roundtrip(t.width, t.height, &t.rgba));
                worst = worst.min(p);
                sum += p;
                n += 1;
                px += (t.width * t.height) as usize;
            }
            println!("BC7 {file}: {n} textures {:.1} Mpx, mean {:.1} dB, worst {:.1} dB, {:.0} ms (with decode)", px as f64 / 1e6, sum / n.max(1) as f64, worst, t0.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
