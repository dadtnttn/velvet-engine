//! A box blur of premultiplied RGBA pixels, three passes each way: close to a gaussian, for soft shadows.

/// Blurs the pixels in place with a box of `radius` pixels, three times horizontally and three times vertically.
pub(crate) fn box_blur(data: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    let mut scratch = vec![0u8; data.len()];
    for _ in 0..3 {
        horizontal(data, &mut scratch, width, height, radius);
        vertical(&scratch, data, width, height, radius);
    }
}

fn horizontal(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    let window = (2 * radius + 1) as u32;
    for y in 0..height {
        let row = y * width * 4;
        for c in 0..4 {
            let at = |x: isize| -> u32 { src[row + x.clamp(0, width as isize - 1) as usize * 4 + c] as u32 };
            let mut sum: u32 = (-(radius as isize)..=radius as isize).map(at).sum();
            for x in 0..width {
                dst[row + x * 4 + c] = ((sum + window / 2) / window) as u8;
                sum += at(x as isize + radius as isize + 1);
                sum -= at(x as isize - radius as isize);
            }
        }
    }
}

fn vertical(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    let window = (2 * radius + 1) as u32;
    for x in 0..width {
        for c in 0..4 {
            let at = |y: isize| -> u32 { src[y.clamp(0, height as isize - 1) as usize * width * 4 + x * 4 + c] as u32 };
            let mut sum: u32 = (-(radius as isize)..=radius as isize).map(at).sum();
            for y in 0..height {
                dst[y * width * 4 + x * 4 + c] = ((sum + window / 2) / window) as u8;
                sum += at(y as isize + radius as isize + 1);
                sum -= at(y as isize - radius as isize);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_dot_spreads_and_keeps_roughly_its_weight() {
        let (w, h) = (21usize, 21usize);
        let mut data = vec![0u8; w * h * 4];
        let centre = (10 * w + 10) * 4;
        data[centre..centre + 4].copy_from_slice(&[255, 255, 255, 255]);
        box_blur(&mut data, w, h, 2);
        assert!(data[centre + 3] < 255 && data[centre + 3] > 0);
        assert!(data[(10 * w + 13) * 4 + 3] > 0, "the dot spreads sideways");
        let total: u32 = data.chunks(4).map(|p| p[3] as u32).sum();
        assert!((200..=320).contains(&total), "weight {total}");
    }
}
