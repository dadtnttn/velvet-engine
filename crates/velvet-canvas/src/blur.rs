//! A box blur of a one-channel mask, three passes each way: close to a gaussian, for soft shadows.

/// Blurs the mask in place with a box of `radius` pixels, three times horizontally and three times vertically.
pub(crate) fn blur_mask(mask: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    let mut line = vec![0u8; width.max(height)];
    for _ in 0..3 {
        for y in 0..height {
            let row = &mut mask[y * width..(y + 1) * width];
            line[..width].copy_from_slice(row);
            box_line(&line[..width], row, radius);
        }
        let mut column = vec![0u8; height];
        for x in 0..width {
            for y in 0..height {
                line[y] = mask[y * width + x];
            }
            box_line(&line[..height], &mut column, radius);
            for y in 0..height {
                mask[y * width + x] = column[y];
            }
        }
    }
}

/// One pass of a box blur along a line, the edges held.
fn box_line(src: &[u8], dst: &mut [u8], radius: usize) {
    let n = src.len();
    let last = n - 1;
    let window = (2 * radius + 1) as u32;
    let mut sum: u32 = src[0] as u32 * (radius as u32 + 1);
    for i in 1..=radius {
        sum += src[i.min(last)] as u32;
    }
    for x in 0..n {
        dst[x] = ((sum + window / 2) / window) as u8;
        sum += src[(x + radius + 1).min(last)] as u32;
        sum -= src[x.saturating_sub(radius)] as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_dot_spreads_and_keeps_roughly_its_weight() {
        let (w, h) = (21usize, 21usize);
        let mut mask = vec![0u8; w * h];
        mask[10 * w + 10] = 255;
        blur_mask(&mut mask, w, h, 2);
        assert!(mask[10 * w + 10] < 255 && mask[10 * w + 10] > 0);
        assert!(mask[10 * w + 13] > 0, "the dot spreads sideways");
        let total: u32 = mask.iter().map(|&a| a as u32).sum();
        assert!((200..=320).contains(&total), "weight {total}");
    }

    #[test]
    fn a_full_mask_stays_full_and_edges_are_held() {
        let mut mask = vec![255u8; 30 * 10];
        blur_mask(&mut mask, 30, 10, 4);
        assert!(mask.iter().all(|&a| a == 255));
    }
}
