//! The scenes' clips: a shot is a folder holding either its frames (`frame_0001.jpg` …, how they
//! are made) or the same frames as one video, `clip.mp4` (H.264, 30 frames a second, a keyframe
//! every 30), with `clip.frames` saying how many there are (tools/clips_to_video.py makes both).
//! The video is what ships: a twelfth of the size for the same picture. The release goes further:
//! `clip.ivf` (AV1, made from the same frames by tools/build_release.py), a quarter of the
//! H.264 again, decoded by rav1d (velvet-avif) on any system; a folder holding it plays it first.
//!
//! The players keep asking for frame paths: [`frames`] gives the folder's frames, or for a video
//! one made-up path per frame (`…/clip.mp4#00012`), and [`load`] reads either. A video is decoded
//! by Windows (Media Foundation) on the thread that asks, onwards from where it last stopped, so a
//! clip played in order costs one frame's decoding per frame.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// The video of a shot's folder and its frame count.
pub(crate) const VIDEO: &str = "clip.mp4";
/// The release's AV1 video of a shot (preferred when there).
pub(crate) const VIDEO_AV1: &str = "clip.ivf";
const COUNT: &str = "clip.frames";
/// Frames a second the videos are made at (only to turn a frame's number into a time).
const FPS: f64 = 30.0;

/// The video a shot folder plays from, when it has one.
fn video_of(dir: &Path) -> Option<(PathBuf, usize)> {
    // QA: `VELVET_CLIP_FRAMES=1` plays the frames even where the video is (to compare the two).
    if std::env::var("VELVET_CLIP_FRAMES").is_ok_and(|v| v == "1")
        && dir.join("frame_0001.jpg").is_file()
    {
        return None;
    }
    let video = [VIDEO_AV1, VIDEO]
        .iter()
        .map(|name| dir.join(name))
        .find(|v| v.is_file())?;
    let count = std::fs::read_to_string(dir.join(COUNT))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some((video, count))
}

/// Does `dir` hold a shot (frames or its video)?
pub(crate) fn is_shot(dir: &Path) -> bool {
    dir.join("frame_0001.jpg").is_file() || video_of(dir).is_some()
}

/// The frames of the shot in `dir`, in order (none when it holds none).
pub(crate) fn frames(dir: &Path) -> Vec<PathBuf> {
    if let Some((video, count)) = video_of(dir) {
        let name = video.to_string_lossy().into_owned();
        return (0..count)
            .map(|i| PathBuf::from(format!("{name}#{i:05}")))
            .collect();
    }
    let mut list: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| {
            d.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "jpg" || x == "png"))
                .collect()
        })
        .unwrap_or_default();
    list.sort();
    list
}

/// A frame of a video named as [`frames`] names it: the video and the frame's number.
fn video_frame(path: &Path) -> Option<(PathBuf, usize)> {
    let text = path.to_str()?;
    let (video, index) = text.rsplit_once('#')?;
    (video.ends_with(".mp4") || video.ends_with(".ivf")).then_some(())?;
    Some((PathBuf::from(video), index.parse().ok()?))
}

/// A frame, from its picture or out of its video (`None` when it cannot be read).
pub(crate) fn load(path: &Path) -> Option<image::RgbaImage> {
    match video_frame(path) {
        Some((video, index)) => decode(&video, index),
        None => velvet_avif::open_image(path).ok().map(|p| p.to_rgba8()),
    }
}

thread_local! {
    /// The AV1 videos this thread has open, the last used first.
    static OPEN_AV1: RefCell<Vec<(PathBuf, velvet_avif::Clip)>> = const { RefCell::new(Vec::new()) };
}

/// A frame of an AV1 video, decoded on this thread (no threads of the decoder's own, so nothing
/// to wait for when the thread ends).
fn decode_av1(video: &Path, index: usize) -> Option<image::RgbaImage> {
    OPEN_AV1.with(|open| {
        let open = &mut *open.borrow_mut();
        let at = match open.iter().position(|(p, _)| p == video) {
            Some(at) => at,
            None => {
                let clip = match velvet_avif::Clip::open(video, 1) {
                    Ok(c) => c,
                    Err(error) => {
                        eprintln!("[clip] {}: {error}", video.display());
                        return None;
                    }
                };
                open.insert(0, (video.to_path_buf(), clip));
                open.truncate(KEEP_OPEN);
                0
            }
        };
        let entry = open.remove(at);
        open.insert(0, entry);
        match open[0].1.frame(index) {
            Ok(frame) => Some(frame),
            Err(error) => {
                eprintln!("[clip] {} #{index}: {error}", video.display());
                None
            }
        }
    })
}

thread_local! {
    /// The videos this thread has open, the last used first (a few: a fade reads two at once).
    static OPEN: RefCell<Open> = const { RefCell::new(Open(Vec::new())) };
}

/// A thread's open videos. Closed by [`close_all`] before the thread ends: let go while the
/// thread is being torn down (under Windows' loader lock), the decoder waits for its own
/// threads and never returns, so whatever is still open then is left as it is.
struct Open(Vec<Reader>);

impl Drop for Open {
    fn drop(&mut self) {
        for reader in self.0.drain(..) {
            std::mem::forget(reader);
        }
    }
}

/// Closes the videos this thread has open (a decoding thread calls it before it ends).
pub(crate) fn close_all() {
    let _ = OPEN.try_with(|open| open.borrow_mut().0.clear());
    let _ = OPEN_AV1.try_with(|open| open.borrow_mut().clear());
}

const KEEP_OPEN: usize = 4;

fn decode(video: &Path, index: usize) -> Option<image::RgbaImage> {
    if video.extension().is_some_and(|e| e == "ivf") {
        return decode_av1(video, index);
    }
    OPEN.with(|open| {
        let open = &mut open.borrow_mut().0;
        let at = match open.iter().position(|r| r.path == video) {
            Some(at) => at,
            None => {
                let reader = match Reader::open(video) {
                    Ok(r) => r,
                    Err(error) => {
                        eprintln!("[clip] {}: {error}", video.display());
                        return None;
                    }
                };
                open.insert(0, reader);
                open.truncate(KEEP_OPEN);
                0
            }
        };
        let mut reader = open.remove(at);
        let frame = reader.frame(index);
        open.insert(0, reader);
        frame
    })
}

#[cfg(windows)]
use windows::Win32::Media::MediaFoundation::*;

/// One video open for reading, where it stands and the last frame it made.
struct Reader {
    path: PathBuf,
    #[cfg(windows)]
    source: IMFSourceReader,
    size: (usize, usize),
    /// Bytes from one row of its pictures to the next.
    pitch: usize,
    /// The time of its first frame (100 ns units; a video can start a little after 0).
    start: i64,
    /// The number of the frame the next read gives (`None` right after a seek: unknown).
    next: Option<usize>,
    last: Option<(usize, image::RgbaImage)>,
}

/// Worker threads a video's decoder gets (QA: `VELVET_CLIP_THREADS`).
#[cfg(windows)]
fn decoder_threads() -> u32 {
    std::env::var("VELVET_CLIP_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
}

#[cfg(windows)]
fn media_foundation() -> windows::core::Result<()> {
    use std::sync::OnceLock;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    static STARTED: OnceLock<windows::core::HRESULT> = OnceLock::new();
    thread_local! {
        static COM: () = {
            // SAFETY: once per thread, before any COM object is made on it; a thread already in
            // another apartment keeps it (the call then fails harmlessly).
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        };
    }
    COM.with(|_| ());
    // SAFETY: MFStartup is counted and may be called from any thread; it is never shut down.
    STARTED
        .get_or_init(
            || match unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) } {
                Ok(()) => windows::core::HRESULT(0),
                Err(e) => e.code(),
            },
        )
        .ok()
}

#[cfg(windows)]
impl Reader {
    fn open(path: &Path) -> windows::core::Result<Self> {
        media_foundation()?;
        let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        // SAFETY: plain Media Foundation calls on objects made here, used on this thread only.
        unsafe {
            let mut attributes: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attributes, 1)?;
            let attributes = attributes.ok_or_else(windows::core::Error::empty)?;
            let url = windows::core::HSTRING::from(path.as_os_str());
            let source = MFCreateSourceReaderFromURL(&url, &attributes)?;
            // Its own pictures (NV12, as the decoder makes them): turned to RGB here, by the
            // same sums the video was made with.
            let wanted = MFCreateMediaType()?;
            wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            wanted.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
            source.SetCurrentMediaType(stream, None, &wanted)?;
            // A leaner decoder: two workers (each keeps its own pictures; with one per core a
            // video open costs ~100 MB, with two ~40 MB and it still decodes a 960² frame in
            // ~5 ms; the low-latency mode would save more but decodes at half the speed).
            let mut raw = std::ptr::null_mut();
            if source
                .GetServiceForStream(
                    stream,
                    &windows::core::GUID::zeroed(),
                    &<IMFTransform as windows::core::Interface>::IID,
                    &mut raw,
                )
                .is_ok()
                && !raw.is_null()
            {
                let decoder = <IMFTransform as windows::core::Interface>::from_raw(raw);
                if let Ok(attributes) = decoder.GetAttributes() {
                    let _ =
                        attributes.SetUINT32(&CODECAPI_AVDecNumWorkerThreads, decoder_threads());
                }
            }
            let got = source.GetCurrentMediaType(stream)?;
            let size = got.GetUINT64(&MF_MT_FRAME_SIZE)?;
            let size = ((size >> 32) as usize, (size & 0xffff_ffff) as usize);
            let pitch = got
                .GetUINT32(&MF_MT_DEFAULT_STRIDE)
                .map_or(size.0, |s| (s as i32).unsigned_abs() as usize)
                .max(size.0);
            let mut reader = Self {
                path: path.to_owned(),
                source,
                size,
                pitch,
                start: 0,
                next: Some(0),
                last: None,
            };
            // Its first frame: where its clock starts, and frame 0 at hand.
            let (start, sample) = reader.read().ok_or_else(windows::core::Error::empty)?;
            reader.start = start;
            reader.next = Some(1);
            reader.last = reader.picture(&sample).map(|p| (0, p));
            Ok(reader)
        }
    }

    /// Frame `index` (the last one past the end).
    fn frame(&mut self, index: usize) -> Option<image::RgbaImage> {
        if let Some((at, picture)) = &self.last {
            if *at == index {
                return Some(picture.clone());
            }
        }
        // Onwards from here when it is a little ahead; else back to the keyframe before it.
        let ahead = self
            .next
            .is_some_and(|next| index >= next && index - next < 45);
        if !ahead {
            // To its time (the last frames' times can lie past what the file calls its length:
            // then a little before, and on from there).
            let time = self.start + (index as f64 * 10_000_000.0 / FPS) as i64;
            let frame = (10_000_000.0 / FPS) as i64;
            let landed = [time, time - self.start, time - 3 * frame, time - 30 * frame]
                .into_iter()
                .any(|t| {
                    let position = windows::core::PROPVARIANT::from(t.max(0));
                    // SAFETY: a seek on the reader this thread owns.
                    unsafe {
                        self.source
                            .SetCurrentPosition(&windows::core::GUID::zeroed(), &position)
                    }
                    .is_ok()
                });
            if !landed {
                return self.last.as_ref().map(|(_, p)| p.clone());
            }
            self.next = None;
        }
        loop {
            let Some((time, sample)) = self.read() else {
                // The end: the last frame stays.
                self.next = None;
                return self.last.as_ref().map(|(_, p)| p.clone());
            };
            let at = self.index_at(time);
            self.next = Some(at + 1);
            if at >= index {
                let picture = self.picture(&sample)?;
                self.last = Some((at, picture.clone()));
                return Some(picture);
            }
        }
    }

    /// The number of the frame shown at `time`.
    fn index_at(&self, time: i64) -> usize {
        ((time - self.start) as f64 * FPS / 10_000_000.0)
            .round()
            .max(0.0) as usize
    }

    /// The next sample and its time (`None` at the end or on an error).
    fn read(&mut self) -> Option<(i64, IMFSample)> {
        let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        loop {
            let mut flags = 0u32;
            let mut time = 0i64;
            let mut sample: Option<IMFSample> = None;
            // SAFETY: a synchronous read on the reader this thread owns, into locals.
            unsafe {
                self.source
                    .ReadSample(
                        stream,
                        0,
                        None,
                        Some(&mut flags),
                        Some(&mut time),
                        Some(&mut sample),
                    )
                    .ok()?;
            }
            if flags & (MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 | MF_SOURCE_READERF_ERROR.0 as u32)
                != 0
            {
                return None;
            }
            if let Some(sample) = sample {
                return Some((time, sample));
            }
        }
    }

    /// A decoded sample as RGBA.
    fn picture(&self, sample: &IMFSample) -> Option<image::RgbaImage> {
        let (w, h) = self.size;
        // SAFETY: the buffer is locked while its bytes are read and unlocked after; the planes'
        // offsets come from its own pitch and length.
        unsafe {
            let buffer = sample.ConvertToContiguousBuffer().ok()?;
            let mut data = std::ptr::null_mut();
            let mut length = 0u32;
            buffer.Lock(&mut data, None, Some(&mut length)).ok()?;
            let bytes = std::slice::from_raw_parts(data, length as usize);
            // The decoder's rows can be wider and its planes taller than the picture
            // (1080 rows are decoded as 1088): NV12 is Y rows then half as many UV rows.
            let pitch = if bytes.len() >= self.pitch * h * 3 / 2 {
                self.pitch
            } else {
                w
            };
            let rows = bytes.len() * 2 / 3 / pitch.max(1);
            let picture = (pitch >= w && rows >= h && bytes.len() >= pitch * rows * 3 / 2)
                .then(|| nv12_to_rgba(bytes, w, h, pitch, pitch * rows));
            let _ = buffer.Unlock();
            picture
        }
    }
}

#[cfg(not(windows))]
impl Reader {
    fn open(_path: &Path) -> Result<Self, &'static str> {
        Err("videos are read with Media Foundation (Windows only)")
    }
    fn frame(&mut self, _index: usize) -> Option<image::RgbaImage> {
        None
    }
}

/// NV12 (BT.601, studio range, as the videos are made) to RGBA, the colour planes filled in
/// between their samples.
fn nv12_to_rgba(
    bytes: &[u8],
    w: usize,
    h: usize,
    pitch: usize,
    uv_start: usize,
) -> image::RgbaImage {
    let mut out = vec![0u8; w * h * 4];
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let chroma = |cx: isize, cy: isize| -> (i32, i32) {
        let cx = cx.clamp(0, cw as isize - 1) as usize;
        let cy = cy.clamp(0, ch as isize - 1) as usize;
        let at = uv_start + cy * pitch + cx * 2;
        (bytes[at] as i32, bytes[at + 1] as i32)
    };
    use rayon_like::rows;
    rows(&mut out, w * 4, |y, row| {
        // The colour sample row above and below this row's centre, and how far between.
        let fy = (y as f32 - 0.5) / 2.0;
        let cy0 = fy.floor() as isize;
        let ty = ((fy - cy0 as f32) * 256.0) as i32;
        for x in 0..w {
            let fx = (x as f32 - 0.5) / 2.0;
            let cx0 = fx.floor() as isize;
            let tx = ((fx - cx0 as f32) * 256.0) as i32;
            let (u00, v00) = chroma(cx0, cy0);
            let (u10, v10) = chroma(cx0 + 1, cy0);
            let (u01, v01) = chroma(cx0, cy0 + 1);
            let (u11, v11) = chroma(cx0 + 1, cy0 + 1);
            let mix = |a: i32, b: i32, c: i32, d: i32| {
                let top = a * (256 - tx) + b * tx;
                let bottom = c * (256 - tx) + d * tx;
                (top * (256 - ty) + bottom * ty + (1 << 15)) >> 16
            };
            let u = mix(u00, u10, u01, u11) - 128;
            let v = mix(v00, v10, v01, v11) - 128;
            let luma = 298 * (bytes[y * pitch + x] as i32 - 16);
            let r = (luma + 409 * v + 128) >> 8;
            let g = (luma - 100 * u - 208 * v + 128) >> 8;
            let b = (luma + 516 * u + 128) >> 8;
            let p = &mut row[x * 4..x * 4 + 4];
            p[0] = r.clamp(0, 255) as u8;
            p[1] = g.clamp(0, 255) as u8;
            p[2] = b.clamp(0, 255) as u8;
            p[3] = 255;
        }
    });
    image::RgbaImage::from_raw(w as u32, h as u32, out).expect("sized above")
}

/// The rows of a picture shared out over a few threads.
mod rayon_like {
    pub(super) fn rows(out: &mut [u8], stride: usize, f: impl Fn(usize, &mut [u8]) + Sync) {
        let height = out.len() / stride.max(1);
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .clamp(1, 8);
        let band = height.div_ceil(threads).max(1);
        // On the engine's kept threads (one made for every frame of a video would leak a little).
        let parts: Vec<(usize, &mut [u8])> = out.chunks_mut(band * stride).enumerate().collect();
        velvet_render::work::each(parts, &|(b, chunk): (usize, &mut [u8])| {
            for (i, row) in chunk.chunks_mut(stride).enumerate() {
                f(b * band + i, row);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_video_frame_is_named_by_its_number() {
        let path = PathBuf::from("data/assets/x/clip.mp4#00012");
        assert_eq!(
            video_frame(&path),
            Some((PathBuf::from("data/assets/x/clip.mp4"), 12))
        );
        assert_eq!(video_frame(Path::new("data/assets/x/frame_0001.jpg")), None);
    }

    #[test]
    fn grey_stays_grey_and_white_white() {
        // 2×2 NV12: Y 16 (black) and 235 (white), neutral colour.
        let bytes = [16u8, 235, 16, 235, 128, 128];
        let rgba = nv12_to_rgba(&bytes, 2, 2, 2, 4);
        assert_eq!(rgba.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(rgba.get_pixel(1, 0).0, [255, 255, 255, 255]);
    }

    /// The video of a shot made from its frames shows the same picture (QA, run by hand:
    /// `cargo test … clip_video -- --ignored` after tools/clips_to_video.py). With
    /// `VELVET_PACKED_RELEASE=<release folder>`, the release's AV1 videos against the project's
    /// frames instead (after tools/build_release.py).
    #[test]
    #[ignore]
    fn every_video_matches_its_frames() {
        let project = crate::exploration3d_state::data_dir().join("assets");
        let release =
            std::env::var_os("VELVET_PACKED_RELEASE").map(|r| PathBuf::from(r).join("data/assets"));
        let root = release.clone().unwrap_or_else(|| project.clone());
        // AV1 at the release's make sits a little further from the frames than the H.264.
        let floor = if release.is_some() { 33.0 } else { 38.0 };
        let mut checked = 0;
        let mut worst = f64::MAX;
        let mut worst_at = String::new();
        for dir in walk(&root) {
            let Some((video, count)) = video_of(&dir) else {
                continue;
            };
            let frames_dir = project.join(dir.strip_prefix(&root).unwrap());
            let jpgs: Vec<PathBuf> = {
                let mut l: Vec<PathBuf> = std::fs::read_dir(&frames_dir)
                    .unwrap()
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x == "jpg"))
                    .collect();
                l.sort();
                l
            };
            if jpgs.is_empty() {
                continue;
            }
            assert_eq!(count, jpgs.len(), "{}", dir.display());
            // In order, then jumping about (back, near the end, the last, the first again).
            let order = [
                0,
                1,
                2,
                count / 2,
                count / 2 + 1,
                count - 1,
                count - 2,
                1,
                count / 3,
                count - 31,
                5,
                0,
            ];
            for i in order.into_iter().filter(|&i| i < count) {
                let a = decode(&video, i).unwrap_or_else(|| panic!("{} #{i}", video.display()));
                let b = image::open(&jpgs[i]).unwrap().to_rgba8();
                assert_eq!(a.dimensions(), b.dimensions());
                let mse: f64 = a
                    .as_raw()
                    .iter()
                    .zip(b.as_raw())
                    .map(|(x, y)| (*x as f64 - *y as f64).powi(2))
                    .sum::<f64>()
                    / a.as_raw().len() as f64;
                let psnr = 10.0 * (255.0f64 * 255.0 / mse.max(1e-9)).log10();
                assert!(psnr > floor, "{} #{i}: {psnr:.1} dB", dir.display());
                if psnr < worst {
                    worst = psnr;
                    worst_at = format!("{} #{i}", dir.display());
                }
            }
            checked += 1;
        }
        close_all();
        assert!(checked > 0);
        eprintln!("{checked} videos match their frames (worst frame {worst:.1} dB, {worst_at})");
    }

    /// How fast a clip plays from its video and from its frames (QA, by hand).
    #[test]
    #[ignore]
    fn video_against_frames_speed() {
        let dir = crate::exploration3d_state::data_dir().join("assets/burdel/nina/escena/bucle");
        let (video, count) = video_of(&dir).expect("converted");
        let t = std::time::Instant::now();
        let opened = Reader::open(&video).map(|_| ()).is_ok();
        let open_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        for i in 0..count {
            decode(&video, i).unwrap();
        }
        let video_ms = t.elapsed().as_secs_f64() * 1000.0 / count as f64;
        let t = std::time::Instant::now();
        for i in 1..=count {
            std::hint::black_box(
                image::open(dir.join(format!("frame_{i:04}.jpg")))
                    .unwrap()
                    .to_rgba8(),
            );
        }
        let jpg_ms = t.elapsed().as_secs_f64() * 1000.0 / count as f64;
        let t = std::time::Instant::now();
        decode(&video, count / 2).unwrap();
        decode(&video, 3).unwrap();
        let seek_ms = t.elapsed().as_secs_f64() * 1000.0 / 2.0;
        let other =
            crate::exploration3d_state::data_dir().join("assets/burdel/nina/escena/intro/clip.mp4");
        let t = std::time::Instant::now();
        let again = Reader::open(&other).map(|_| ()).is_ok();
        eprintln!(
            "a second video opened {again} in {:.0} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
        let before = velvet_qa::private_mb();
        let mut readers = Vec::new();
        for name in ["bucle", "bucle_rapido", "climax", "intro", "relax"] {
            readers.push(Reader::open(&dir.with_file_name(name).join(VIDEO)).unwrap());
        }
        eprintln!(
            "5 readers: {} MB",
            velvet_qa::private_mb() as i64 - before as i64
        );
        drop(readers);
        close_all();
        eprintln!("open {opened} {open_ms:.0} ms; per frame: video {video_ms:.1} ms, jpg {jpg_ms:.1} ms; a jump {seek_ms:.0} ms");
    }

    fn walk(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_owned()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                if e.path().is_dir() {
                    stack.push(e.path());
                }
            }
            out.push(dir);
        }
        out
    }
}
