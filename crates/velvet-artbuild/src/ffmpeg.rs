//! Runs ffmpeg for the two formats that no Rust encoder writes here: WebP (libwebp) and AVIF
//! (libaom-av1). The picture goes in as raw RGBA on standard input, so no intermediate file is made,
//! and the encoded file is written to the path that the caller gives.

use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use image::RgbaImage;

use crate::config::Quality;
use crate::output_format::OutputFormat;

/// The AV1 speed setting for AVIF: 6 is quick, and the files stay small.
pub const AVIF_CPU_USED: u8 = 6;

/// The colour metadata that the colour stream of an AVIF carries: BT.709 primaries, the sRGB transfer
/// curve, BT.601 matrix and limited range. These are the conversions that the encoder is given, so the
/// file says what the pixels mean.
const AVIF_COLOUR: &str =
    "setparams=color_primaries=bt709:color_trc=iec61966-2-1:colorspace=bt470bg:range=tv";

/// The filter graph that splits an RGBA picture into a colour stream and an alpha stream. The alpha
/// stream is the alpha channel as one plane, at full range, which is what the AVIF alpha item needs.
const AVIF_WITH_ALPHA: &str = "[0:v]split=2[c][a];[c]format=yuv420p,setparams=color_primaries=bt709:color_trc=iec61966-2-1:colorspace=bt470bg:range=tv[color];[a]alphaextract,format=gray,setparams=colorspace=bt709:range=pc[alpha]";

/// The encoders that an ffmpeg build has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Encoders {
    /// True when the build has the `libwebp` encoder.
    pub libwebp: bool,
    /// True when the build has the `libaom-av1` encoder.
    pub libaom_av1: bool,
}

/// Asks ffmpeg which encoders it has.
///
/// # Errors
/// Returns the reason when ffmpeg cannot be run, such as when it is not on the PATH.
pub fn probe(program: &OsStr) -> Result<Encoders, String> {
    let output = Command::new(program)
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => "ffmpeg is not on the PATH".to_string(),
            _ => format!("cannot run ffmpeg: {error}"),
        })?;
    if !output.status.success() {
        return Err("ffmpeg could not list its encoders".to_string());
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    let has = |name: &str| {
        listing
            .lines()
            .any(|line| line.split_whitespace().nth(1) == Some(name))
    };
    Ok(Encoders {
        libwebp: has("libwebp"),
        libaom_av1: has("libaom-av1"),
    })
}

/// The CRF that libaom-av1 takes for an AVIF quality from 1 to 100. Quality 100 gives CRF 0 and
/// quality 1 gives CRF 63, the range of the encoder, in between in proportion, rounded to the nearest.
pub fn crf_for_quality(quality: u8) -> u8 {
    let headroom = u32::from(100u8.saturating_sub(quality));
    ((headroom * 63 + 50) / 100) as u8
}

/// Appends fixed arguments to a list.
fn push_all(args: &mut Vec<String>, items: &[&str]) {
    args.extend(items.iter().map(|item| item.to_string()));
}

/// The arguments of the WebP encode, up to the output path, which the caller adds last.
pub fn webp_args(width: u32, height: u32, quality: u8) -> Vec<String> {
    let mut args = input_args(width, height);
    push_all(
        &mut args,
        &["-frames:v", "1", "-c:v", "libwebp", "-quality"],
    );
    args.push(quality.to_string());
    push_all(&mut args, &["-pix_fmt", "yuva420p", "-f", "webp"]);
    args
}

/// The arguments of the AVIF encode, up to the output path, which the caller adds last. A picture
/// with any transparent pixel gets a second stream for its alpha; an opaque picture has one stream.
pub fn avif_args(width: u32, height: u32, quality: u8, has_alpha: bool) -> Vec<String> {
    let crf = crf_for_quality(quality).to_string();
    let cpu_used = AVIF_CPU_USED.to_string();
    let mut args = input_args(width, height);
    if has_alpha {
        push_all(&mut args, &["-filter_complex", AVIF_WITH_ALPHA]);
        for (index, label) in [(0, "[color]"), (1, "[alpha]")] {
            push_all(&mut args, &["-map", label]);
            args.push(format!("-c:v:{index}"));
            push_all(&mut args, &["libaom-av1", "-crf"]);
            args.push(crf.clone());
            push_all(&mut args, &["-b:v", "0", "-cpu-used"]);
            args.push(cpu_used.clone());
            push_all(&mut args, &["-still-picture", "1"]);
        }
    } else {
        push_all(&mut args, &["-vf"]);
        args.push(format!("format=yuv420p,{AVIF_COLOUR}"));
        push_all(&mut args, &["-c:v", "libaom-av1", "-crf"]);
        args.push(crf);
        push_all(&mut args, &["-b:v", "0", "-cpu-used"]);
        args.push(cpu_used);
        push_all(&mut args, &["-still-picture", "1"]);
    }
    push_all(&mut args, &["-frames:v", "1", "-f", "avif"]);
    args
}

/// The arguments that read one raw RGBA frame of the given size from standard input.
fn input_args(width: u32, height: u32) -> Vec<String> {
    let mut args = Vec::new();
    push_all(
        &mut args,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostats",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-video_size",
        ],
    );
    args.push(format!("{width}x{height}"));
    push_all(&mut args, &["-i", "pipe:0"]);
    args
}

/// Encodes a picture with ffmpeg into `output`, in the format and at the quality that the caller gives.
///
/// # Errors
/// Returns ffmpeg's own message when it fails, or a short reason when it leaves no file behind.
pub fn encode(
    program: &OsStr,
    format: OutputFormat,
    picture: &RgbaImage,
    quality: Quality,
    output: &Path,
) -> Result<(), String> {
    let (width, height) = picture.dimensions();
    let has_alpha = picture.pixels().any(|pixel| pixel.0[3] != 255);
    let args = match format {
        OutputFormat::Webp => webp_args(width, height, quality.webp),
        OutputFormat::Avif => avif_args(width, height, quality.avif, has_alpha),
        OutputFormat::Png => {
            return Err("PNG is written by the image crate, not by ffmpeg".to_string())
        }
    };
    run_with_input(program, &args, output, picture.as_raw())
}

/// Runs ffmpeg with `args`, pipes `raw` to its standard input, and checks that `output` was written.
fn run_with_input(
    program: &OsStr,
    args: &[String],
    output: &Path,
    raw: &[u8],
) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .arg(output)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start ffmpeg: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg did not open its input")?;
    let (status, stderr, written) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            let result = stdin.write_all(raw);
            drop(stdin);
            result
        });
        let finished = child.wait_with_output();
        let written = writer
            .join()
            .unwrap_or_else(|_| Err(std::io::Error::other("the input thread panicked")));
        finished.map(|output| (output.status, output.stderr, written))
    })
    .map_err(|error| format!("ffmpeg did not finish: {error}"))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg failed: {}",
            first_lines(&String::from_utf8_lossy(&stderr), 4)
        ));
    }
    if let Err(error) = written {
        return Err(format!("ffmpeg did not read the picture: {error}"));
    }
    match std::fs::metadata(output) {
        Ok(meta) if meta.len() > 0 => Ok(()),
        _ => Err("ffmpeg wrote no file".to_string()),
    }
}

/// The first `count` non-empty lines of a text, joined with spaces, for an error message.
fn first_lines(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(count)
        .collect();
    if lines.is_empty() {
        "no message".to_string()
    } else {
        lines.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crf_follows_the_quality_scale() {
        assert_eq!(crf_for_quality(100), 0);
        assert_eq!(crf_for_quality(60), 25);
        assert_eq!(crf_for_quality(1), 62);
    }

    #[test]
    fn webp_arguments_read_raw_rgba_and_write_a_lossy_webp_with_alpha() {
        let args = webp_args(64, 32, 85).join(" ");
        assert!(args.contains("-video_size 64x32 -i pipe:0"), "{args}");
        assert!(
            args.contains("-c:v libwebp -quality 85 -pix_fmt yuva420p -f webp"),
            "{args}"
        );
    }

    #[test]
    fn avif_arguments_use_two_streams_only_when_there_is_alpha() {
        let with_alpha = avif_args(64, 32, 60, true).join(" ");
        assert!(with_alpha.contains("-filter_complex"), "{with_alpha}");
        assert!(
            with_alpha.contains("-map [color] -c:v:0 libaom-av1 -crf 25"),
            "{with_alpha}"
        );
        assert!(
            with_alpha.contains("-map [alpha] -c:v:1 libaom-av1 -crf 25"),
            "{with_alpha}"
        );
        let opaque = avif_args(64, 32, 60, false).join(" ");
        assert!(!opaque.contains("-filter_complex"), "{opaque}");
        assert!(opaque.contains("-c:v libaom-av1 -crf 25"), "{opaque}");
    }

    #[test]
    fn first_lines_are_short_and_never_empty() {
        assert_eq!(first_lines("\n  a \n\nb\nc\nd\ne", 2), "a b");
        assert_eq!(first_lines("   ", 3), "no message");
    }
}
