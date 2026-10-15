//! Decodes against other decoders' output for the same files (tests/data, made with Pillow's
//! libavif and ffmpeg's libaom from synthetic pictures).

use std::path::{Path, PathBuf};

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

fn reference(name: &str) -> image::RgbaImage {
    image::open(data(name)).unwrap().to_rgba8()
}

fn psnr(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions());
    let mse = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| (*x as f64 - *y as f64).powi(2))
        .sum::<f64>()
        / a.as_raw().len() as f64;
    10.0 * (255.0f64 * 255.0 / mse.max(1e-9)).log10()
}

#[test]
fn an_avif_with_alpha_decodes_as_libavif_does() {
    let ours = velvet_avif::decode_avif(&std::fs::read(data("sprite444.avif")).unwrap()).unwrap();
    let theirs = reference("sprite444.pillow.png");
    assert_eq!(ours.dimensions(), theirs.dimensions());
    for (a, b) in ours.pixels().zip(theirs.pixels()) {
        assert_eq!(a.0[3], b.0[3], "alpha");
        if b.0[3] > 0 {
            for c in 0..3 {
                assert!((a.0[c] as i32 - b.0[c] as i32).abs() <= 1, "{a:?} {b:?}");
            }
        }
    }
    // The fully transparent columns stay so.
    assert_eq!(ours.get_pixel(5, 40).0[3], 0);
}

#[test]
fn a_subsampled_avif_decodes_as_libavif_does() {
    let ours = velvet_avif::decode_avif(&std::fs::read(data("picture420.avif")).unwrap()).unwrap();
    let theirs = reference("picture420.pillow.png");
    for (a, b) in ours.pixels().zip(theirs.pixels()) {
        assert_eq!(a.0[3], 255);
        for c in 0..3 {
            assert!((a.0[c] as i32 - b.0[c] as i32).abs() <= 1, "{a:?} {b:?}");
        }
    }
}

#[test]
fn the_image_crate_reads_avif_by_its_bytes_once_hooked() {
    velvet_avif::register_image_hooks();
    let bytes = std::fs::read(data("sprite444.avif")).unwrap();
    assert!(velvet_avif::is_avif(&bytes));
    let via_image = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(via_image, velvet_avif::decode_avif(&bytes).unwrap());
    // Under a .png name too (the release keeps the names).
    let dir = std::env::temp_dir().join(format!("velvet-avif-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let named = dir.join("cg.png");
    std::fs::write(&named, &bytes).unwrap();
    assert_eq!(velvet_avif::open_image(&named).unwrap().to_rgba8(), via_image);
    std::fs::remove_dir_all(&dir).ok();
    assert!(!velvet_avif::is_avif(b"\x89PNG\r\n\x1a\n0000"));
}

#[test]
fn a_clip_gives_every_frame_in_order_and_jumping_about() {
    let mut clip = velvet_avif::Clip::open(&data("clip.ivf"), 1).unwrap();
    assert_eq!(clip.len(), 12);
    let in_order: Vec<image::RgbaImage> = (0..clip.len()).map(|i| clip.frame(i).unwrap()).collect();
    for (i, frame) in in_order.iter().enumerate() {
        // ffmpeg fills chroma in its own way: as close to the source, not equal to it.
        let source = reference(&format!("clip_{i:02}.source.png"));
        let ours = psnr(frame, &source);
        let theirs = psnr(&reference(&format!("clip_{i:02}.ffmpeg.png")), &source);
        assert!(ours > theirs - 0.5, "frame {i}: {ours:.1} dB against ffmpeg's {theirs:.1}");
    }
    // Back, forward past a keyframe, the same twice, past the end: the same pictures.
    let mut other = velvet_avif::Clip::open(&data("clip.ivf"), 2).unwrap();
    for i in [7, 2, 11, 11, 0, 6, 5, 9, 3] {
        assert_eq!(other.frame(i).unwrap(), in_order[i], "frame {i}");
    }
    assert_eq!(other.frame(40).unwrap(), in_order[11]);
}
