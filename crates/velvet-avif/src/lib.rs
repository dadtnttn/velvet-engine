//! AV1 decoding for Velvet Engine, on rav1d (the Rust port of dav1d; no C, no system codec, the
//! same on Windows and Android): AVIF pictures (colour and alpha) and AV1 clips stored as IVF.
//!
//! - [`decode_avif`] turns an `.avif` file's bytes into RGBA.
//! - [`register_image_hooks`] teaches the `image` crate to recognise AVIF by its bytes, so every
//!   `image::load_from_memory` / `with_guessed_format` in the program decodes it as any other
//!   picture (whatever the file is called).
//! - [`Clip`] reads an AV1 clip frame by frame (in order, or jumping back to the keyframe before
//!   the one asked for).
//!
//! YUV is turned to RGB by the matrix and range the stream itself declares (BT.601, BT.709,
//! BT.2020 or identity; studio or full range); chroma subsampled 4:2:0 / 4:2:2 is filled in
//! between its samples.

use std::io::{Cursor, Read};
use std::path::Path;
use std::ptr::NonNull;

use image::RgbaImage;
use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::headers::{
    DAV1D_MC_BT2020_CL, DAV1D_MC_BT2020_NCL, DAV1D_MC_BT709, DAV1D_MC_IDENTITY, DAV1D_PIXEL_LAYOUT_I400,
    DAV1D_PIXEL_LAYOUT_I420, DAV1D_PIXEL_LAYOUT_I422,
};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_flush, dav1d_get_picture,
    dav1d_open, dav1d_picture_unref, dav1d_send_data,
};

/// What can go wrong reading AV1.
#[derive(Debug)]
pub enum Error {
    /// The container (AVIF or IVF) is not as expected.
    Container(String),
    /// The decoder refused the data (its error code).
    Decode(i32),
    /// The picture is of a kind this does not turn to RGB (more than 8 bits a sample).
    Unsupported(&'static str),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Container(why) => write!(f, "AV1 container: {why}"),
            Error::Decode(code) => write!(f, "AV1 decoder error {code}"),
            Error::Unsupported(why) => write!(f, "AV1: {why}"),
            Error::Io(e) => write!(f, "AV1 read: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The decoder's "try again" (it wants more data, or has a picture to give first).
fn again(code: i32) -> bool {
    code == -(libc_eagain())
}

const fn libc_eagain() -> i32 {
    // EAGAIN on every platform rav1d builds for (Windows' CRT, Linux, Android, macOS).
    if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        35
    } else {
        11
    }
}

/// One rav1d decoder.
struct Decoder {
    ctx: Option<Dav1dContext>,
}

// SAFETY: the context is only used through `&mut self` (one thread at a time); rav1d's own
// threads are managed inside it.
unsafe impl Send for Decoder {}

impl Decoder {
    /// `threads`: 0 lets rav1d choose. Pictures come out as soon as they are decoded (no frame
    /// threading delay), in order.
    fn new(threads: i32) -> Result<Self> {
        let mut settings = std::mem::MaybeUninit::<Dav1dSettings>::uninit();
        // SAFETY: `dav1d_default_settings` writes every field of the settings it is given.
        let mut settings = unsafe {
            dav1d_default_settings(NonNull::new_unchecked(settings.as_mut_ptr()));
            settings.assume_init()
        };
        settings.n_threads = threads;
        settings.max_frame_delay = 1;
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: both pointers are to live locals.
        let code = unsafe { dav1d_open(Some(NonNull::from(&mut ctx)), Some(NonNull::from(&mut settings))) }.0;
        if code < 0 || ctx.is_none() {
            return Err(Error::Decode(code));
        }
        Ok(Self { ctx })
    }

    /// Gives the decoder one temporal unit (the OBUs of one shown frame).
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        let mut data = Dav1dData::default();
        // SAFETY: `data` is a live local; the buffer it gets is `bytes.len()` long and filled here.
        unsafe {
            let ptr = dav1d_data_create(Some(NonNull::from(&mut data)), bytes.len());
            if ptr.is_null() {
                return Err(Error::Decode(-12));
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }
        loop {
            // SAFETY: the context is open; `data` is valid.
            let code = unsafe { dav1d_send_data(self.ctx.clone(), Some(NonNull::from(&mut data))) }.0;
            if code >= 0 && data.sz == 0 {
                break;
            }
            if code < 0 && !again(code) {
                // SAFETY: `data` is ours to release.
                unsafe { dav1d_data_unref(Some(NonNull::from(&mut data))) };
                return Err(Error::Decode(code));
            }
            if data.sz == 0 {
                break;
            }
            // The decoder is full: a picture must be taken out first. The caller takes them,
            // so here the rest of the data is held until there is room (rare with delay 1).
            if again(code) {
                // SAFETY: as above.
                unsafe { dav1d_data_unref(Some(NonNull::from(&mut data))) };
                return Err(Error::Decode(code));
            }
        }
        Ok(())
    }

    /// The next decoded picture, if one is ready.
    fn take(&mut self) -> Result<Option<Picture>> {
        let mut pic = Dav1dPicture::default();
        // SAFETY: the context is open; `pic` is a live local the decoder writes.
        let code = unsafe { dav1d_get_picture(self.ctx.clone(), Some(NonNull::from(&mut pic))) }.0;
        if code >= 0 {
            return Ok(Some(Picture(pic)));
        }
        if again(code) {
            return Ok(None);
        }
        Err(Error::Decode(code))
    }

    /// Forgets everything queued (before jumping to another keyframe).
    fn flush(&mut self) {
        // SAFETY: the context is open.
        unsafe { dav1d_flush(self.ctx.clone().expect("open")) };
    }

    /// One unit in, its picture out (sends, then asks; asks again for a picture held back).
    fn decode(&mut self, bytes: &[u8]) -> Result<Option<Picture>> {
        self.send(bytes)?;
        for _ in 0..4 {
            if let Some(p) = self.take()? {
                return Ok(Some(p));
            }
        }
        Ok(None)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // SAFETY: closes the context opened in `new` (it sets the option to `None`).
        unsafe { dav1d_close(Some(NonNull::from(&mut self.ctx))) };
    }
}

/// A decoded picture (released when dropped).
struct Picture(Dav1dPicture);

impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: the picture came from `dav1d_get_picture` and is released once.
        unsafe { dav1d_picture_unref(Some(NonNull::from(&mut self.0))) };
    }
}

/// The YUV → RGB sums of a stream.
#[derive(Clone, Copy)]
struct ColorMath {
    identity: bool,
    kr: f32,
    kb: f32,
    full: bool,
}

impl Picture {
    fn size(&self) -> (usize, usize) {
        (self.0.p.w.max(0) as usize, self.0.p.h.max(0) as usize)
    }

    fn math(&self) -> ColorMath {
        // SAFETY: a decoded picture carries its sequence header.
        let seq = self.0.seq_hdr.map(|s| unsafe { s.as_ref() });
        let (mtrx, range) = seq.map_or((2, 0), |s| (s.mtrx, s.color_range));
        let (kr, kb) = match mtrx {
            m if m == DAV1D_MC_BT709 => (0.2126, 0.0722),
            m if m == DAV1D_MC_BT2020_NCL || m == DAV1D_MC_BT2020_CL => (0.2627, 0.0593),
            // BT.601 (BT470BG, SMPTE 170M) and unspecified, as other decoders read it.
            _ => (0.299, 0.114),
        };
        ColorMath { identity: mtrx == DAV1D_MC_IDENTITY, kr, kb, full: range != 0 }
    }

    /// A plane's rows: (data, stride).
    fn plane(&self, i: usize) -> Option<(*const u8, isize)> {
        let data = self.0.data[i]?;
        let stride = if i == 0 { self.0.stride[0] } else { self.0.stride[1] };
        Some((data.as_ptr() as *const u8, stride as isize))
    }

    /// The luma (or grey) samples, row by row, for an alpha plane.
    fn luma(&self) -> Result<Vec<u8>> {
        if self.0.p.bpc != 8 {
            return Err(Error::Unsupported("only 8-bit pictures"));
        }
        let (w, h) = self.size();
        let (y, ys) = self.plane(0).ok_or(Error::Unsupported("no luma plane"))?;
        let mut out = Vec::with_capacity(w * h);
        for row in 0..h {
            // SAFETY: the plane holds `h` rows of at least `w` bytes, `ys` apart.
            let line = unsafe { std::slice::from_raw_parts(y.offset(row as isize * ys), w) };
            out.extend_from_slice(line);
        }
        Ok(out)
    }

    /// The picture as RGBA (opaque).
    fn to_rgba(&self) -> Result<RgbaImage> {
        if self.0.p.bpc != 8 {
            return Err(Error::Unsupported("only 8-bit pictures"));
        }
        let (w, h) = self.size();
        let layout = self.0.p.layout;
        let math = self.math();
        let (y, ys) = self.plane(0).ok_or(Error::Unsupported("no luma plane"))?;
        let mut out = vec![0u8; w * h * 4];
        if layout == DAV1D_PIXEL_LAYOUT_I400 {
            for row in 0..h {
                // SAFETY: as in `luma`.
                let line = unsafe { std::slice::from_raw_parts(y.offset(row as isize * ys), w) };
                for (x, &v) in line.iter().enumerate() {
                    let g = if math.full { v } else { (((v as f32 - 16.0) * 255.0 / 219.0).round()).clamp(0.0, 255.0) as u8 };
                    out[(row * w + x) * 4..(row * w + x) * 4 + 4].copy_from_slice(&[g, g, g, 255]);
                }
            }
            return Ok(RgbaImage::from_raw(w as u32, h as u32, out).expect("sized"));
        }
        let (u, us) = self.plane(1).ok_or(Error::Unsupported("no chroma"))?;
        let (v, _) = self.plane(2).ok_or(Error::Unsupported("no chroma"))?;
        let (sx, sy) = match layout {
            l if l == DAV1D_PIXEL_LAYOUT_I420 => (1usize, 1usize),
            l if l == DAV1D_PIXEL_LAYOUT_I422 => (1, 0),
            _ => (0, 0),
        };
        let (cw, ch) = ((w + sx) >> sx, (h + sy) >> sy);
        // The sums, in fixed point (16 fractional bits).
        let (ys_k, yo, cs_k) = if math.full { (1.0f32, 0.0f32, 1.0f32) } else { (255.0 / 219.0, 16.0, 255.0 / 224.0) };
        let kr = math.kr;
        let kb = math.kb;
        let kg = 1.0 - kr - kb;
        let fx = |v: f32| (v * 65536.0).round() as i32;
        let (cy, crr, cbb, cgr, cgb) = (
            fx(ys_k),
            fx(2.0 * (1.0 - kr) * cs_k),
            fx(2.0 * (1.0 - kb) * cs_k),
            fx(-2.0 * (1.0 - kr) * kr / kg * cs_k),
            fx(-2.0 * (1.0 - kb) * kb / kg * cs_k),
        );
        let yo = yo as i32;
        // Where each column / row sits among the chroma samples (centred between them when
        // subsampled) and how far towards the next: worked out once, not per pixel.
        let place = |n: usize, sub: usize, limit: usize| -> Vec<(usize, usize, i32)> {
            (0..n)
                .map(|i| {
                    if sub == 0 {
                        return (i.min(limit - 1), i.min(limit - 1), 0);
                    }
                    let f = (i as f32 - 0.5) / 2.0;
                    let c = f.floor();
                    let t = ((f - c) * 256.0) as i32;
                    let c0 = (c as isize).clamp(0, limit as isize - 1) as usize;
                    let c1 = (c as isize + 1).clamp(0, limit as isize - 1) as usize;
                    (c0, c1, t)
                })
                .collect()
        };
        let columns = place(w, sx, cw);
        let rows = place(h, sy, ch);
        let subsampled = sx != 0 || sy != 0;
        for (row, &(r0, r1, ty)) in rows.iter().enumerate() {
            // SAFETY: every row index is inside its plane (`h` luma rows, `ch` chroma rows).
            let (yl, u0, u1, v0, v1) = unsafe {
                (
                    std::slice::from_raw_parts(y.offset(row as isize * ys), w),
                    std::slice::from_raw_parts(u.offset(r0 as isize * us), cw),
                    std::slice::from_raw_parts(u.offset(r1 as isize * us), cw),
                    std::slice::from_raw_parts(v.offset(r0 as isize * us), cw),
                    std::slice::from_raw_parts(v.offset(r1 as isize * us), cw),
                )
            };
            let line = &mut out[row * w * 4..(row + 1) * w * 4];
            for (x, &(c0, c1, tx)) in columns.iter().enumerate() {
                let (uv, vv) = if subsampled {
                    let mix = |a: &[u8], b: &[u8]| -> i32 {
                        let top = a[c0] as i32 * (256 - tx) + a[c1] as i32 * tx;
                        let bottom = b[c0] as i32 * (256 - tx) + b[c1] as i32 * tx;
                        (top * (256 - ty) + bottom * ty + (1 << 15)) >> 16
                    };
                    (mix(u0, u1), mix(v0, v1))
                } else {
                    (u0[c0] as i32, v0[c0] as i32)
                };
                let yv = yl[x] as i32;
                let (r, g, b) = if math.identity {
                    // GBR: Y is green, U blue, V red.
                    (vv, yv, uv)
                } else {
                    let l = (yv - yo) * cy;
                    let (pb, pr) = (uv - 128, vv - 128);
                    (
                        (l + crr * pr + (1 << 15)) >> 16,
                        (l + cgr * pr + cgb * pb + (1 << 15)) >> 16,
                        (l + cbb * pb + (1 << 15)) >> 16,
                    )
                };
                let px = &mut line[x * 4..x * 4 + 4];
                px[0] = r.clamp(0, 255) as u8;
                px[1] = g.clamp(0, 255) as u8;
                px[2] = b.clamp(0, 255) as u8;
                px[3] = 255;
            }
        }
        Ok(RgbaImage::from_raw(w as u32, h as u32, out).expect("sized"))
    }
}

/// Decodes one AV1 still (an AVIF item's data).
fn decode_still(obus: &[u8]) -> Result<Picture> {
    let mut decoder = Decoder::new(0)?;
    if let Some(p) = decoder.decode(obus)? {
        return Ok(p);
    }
    Err(Error::Container("the item gave no picture".into()))
}

/// Is `bytes` an AVIF file (an ISO-BMFF `ftyp` box with an AVIF brand)?
pub fn is_avif(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && (&bytes[8..12] == b"avif" || &bytes[8..12] == b"avis")
}

/// An AVIF picture as RGBA (its alpha plane, if any, as alpha; premultiplied alpha undone).
pub fn decode_avif(bytes: &[u8]) -> Result<RgbaImage> {
    let parsed = avif_parse::read_avif(&mut Cursor::new(bytes)).map_err(|e| Error::Container(e.to_string()))?;
    let colour = decode_still(&parsed.primary_item)?;
    let mut rgba = colour.to_rgba()?;
    if let Some(alpha_data) = parsed.alpha_item.as_ref() {
        let alpha = decode_still(alpha_data)?;
        let (aw, ah) = alpha.size();
        if (aw as u32, ah as u32) != rgba.dimensions() {
            return Err(Error::Container("alpha plane of another size".into()));
        }
        let full = alpha.math().full;
        let a = alpha.luma()?;
        for (px, &av) in rgba.pixels_mut().zip(&a) {
            let av = if full { av } else { (((av as f32 - 16.0) * 255.0 / 219.0).round()).clamp(0.0, 255.0) as u8 };
            px.0[3] = av;
            if parsed.premultiplied_alpha && av > 0 && av < 255 {
                for c in 0..3 {
                    px.0[c] = ((px.0[c] as u32 * 255 + av as u32 / 2) / av as u32).min(255) as u8;
                }
            }
        }
    }
    Ok(rgba)
}

/// A decoder the `image` crate can use for AVIF (decoded at once when made).
struct AvifImage(RgbaImage);

impl image::ImageDecoder for AvifImage {
    fn dimensions(&self) -> (u32, u32) {
        self.0.dimensions()
    }

    fn color_type(&self) -> image::ColorType {
        image::ColorType::Rgba8
    }

    fn read_image(self, buf: &mut [u8]) -> image::ImageResult<()> {
        buf.copy_from_slice(self.0.as_raw());
        Ok(())
    }

    fn read_image_boxed(self: Box<Self>, buf: &mut [u8]) -> image::ImageResult<()> {
        (*self).read_image(buf)
    }
}

/// Teaches the `image` crate AVIF: recognised by its bytes (any file name) wherever it guesses a
/// format (`load_from_memory`, `ImageReader::with_guessed_format`). Once is enough; again is
/// harmless.
pub fn register_image_hooks() {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| {
        const EXT: &str = "velvet-avif";
        image::hooks::register_format_detection_hook(
            EXT.into(),
            b"\0\0\0\0ftypavif",
            Some(b"\0\0\0\0\xff\xff\xff\xff\xff\xff\xff\xff"),
        );
        image::hooks::register_decoding_hook(
            EXT.into(),
            Box::new(|mut reader| {
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).map_err(image::ImageError::IoError)?;
                let rgba = decode_avif(&bytes).map_err(|e| {
                    image::ImageError::Decoding(image::error::DecodingError::new(
                        image::error::ImageFormatHint::Name("AVIF".into()),
                        e.to_string(),
                    ))
                })?;
                Ok(Box::new(AvifImage(rgba)))
            }),
        );
    });
}

/// A picture file of any kind the program reads, AVIF included, whatever its name says.
pub fn open_image(path: &Path) -> image::ImageResult<image::DynamicImage> {
    register_image_hooks();
    let bytes = std::fs::read(path).map_err(image::ImageError::IoError)?;
    image::load_from_memory(&bytes)
}

/// An AV1 clip in an IVF file: its frames (temporal units) and which are keyframes.
pub struct Clip {
    bytes: Vec<u8>,
    /// Each frame's (offset, length) in `bytes`.
    frames: Vec<(usize, usize)>,
    keyframes: Vec<usize>,
    decoder: Decoder,
    /// The number of the frame the next decode gives (None: unknown, after a jump).
    next: Option<usize>,
    last: Option<(usize, RgbaImage)>,
}

/// Does this temporal unit begin a keyframe (does it carry a sequence header, as encoders put
/// before every keyframe)?
fn starts_keyframe(tu: &[u8]) -> bool {
    let mut i = 0;
    while i < tu.len() {
        let header = tu[i];
        let obu_type = (header >> 3) & 0x0f;
        let has_ext = header & 0x04 != 0;
        let has_size = header & 0x02 != 0;
        if obu_type == 1 {
            return true;
        }
        let mut j = i + 1 + usize::from(has_ext);
        if !has_size {
            return false;
        }
        // leb128 size
        let mut size = 0usize;
        let mut shift = 0;
        loop {
            let Some(&b) = tu.get(j) else { return false };
            size |= ((b & 0x7f) as usize) << shift;
            j += 1;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        i = j + size;
    }
    false
}

impl Clip {
    /// Opens an IVF file of AV1, decoded with `threads` threads of the decoder's own (1: on the
    /// asking thread alone, none made).
    pub fn open(path: &Path, threads: i32) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.read_to_end(&mut bytes)?;
        Self::from_bytes(bytes, threads)
    }

    pub fn from_bytes(bytes: Vec<u8>, threads: i32) -> Result<Self> {
        if bytes.len() < 32 || &bytes[0..4] != b"DKIF" || &bytes[8..12] != b"AV01" {
            return Err(Error::Container("not an AV1 IVF file".into()));
        }
        let header = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
        let mut at = header;
        let mut frames = Vec::new();
        while at + 12 <= bytes.len() {
            let size = u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4")) as usize;
            let start = at + 12;
            if start + size > bytes.len() {
                break;
            }
            frames.push((start, size));
            at = start + size;
        }
        if frames.is_empty() {
            return Err(Error::Container("no frames".into()));
        }
        let keyframes: Vec<usize> = frames
            .iter()
            .enumerate()
            .filter(|(_, &(o, n))| starts_keyframe(&bytes[o..o + n]))
            .map(|(i, _)| i)
            .collect();
        let keyframes = if keyframes.first() == Some(&0) { keyframes } else { vec![0] };
        Ok(Self { bytes, frames, keyframes, decoder: Decoder::new(threads)?, next: Some(0), last: None })
    }

    /// How many frames it has.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Frame `index` as RGBA (the last one past the end).
    pub fn frame(&mut self, index: usize) -> Result<RgbaImage> {
        let index = index.min(self.frames.len() - 1);
        if let Some((at, picture)) = &self.last {
            if *at == index {
                return Ok(picture.clone());
            }
        }
        // On from here when it is ahead and no keyframe lies between; else from the keyframe
        // before it.
        let key = *self.keyframes.iter().rev().find(|&&k| k <= index).unwrap_or(&0);
        let from = match self.next {
            Some(next) if next <= index && next > key => next,
            Some(next) if next == key => key,
            _ => {
                self.decoder.flush();
                key
            }
        };
        let mut picture = None;
        for i in from..=index {
            let (o, n) = self.frames[i];
            let tu = self.bytes[o..o + n].to_vec();
            let got = self.decoder.decode(&tu)?;
            self.next = Some(i + 1);
            if i == index {
                picture = got;
            }
        }
        let rgba = picture.ok_or(Error::Container("the frame gave no picture".into()))?.to_rgba()?;
        self.last = Some((index, rgba.clone()));
        Ok(rgba)
    }
}
