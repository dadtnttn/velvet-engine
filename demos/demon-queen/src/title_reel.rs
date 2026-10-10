//! The title's moving places: clips of frames (`data/assets/title/reel/<clip>/frame_NNNN.jpg`,
//! 1920 × 1080, 24 a second, drawn 1:1): each place's idle loop (`trono`, `balcon`, `alcoba`,
//! `puertas`), the variants of a loop (`trono#1`, `trono#2`…: she does something, then rests; they
//! start and end on the same frame as the plain loop, so they follow one another seamlessly), and
//! the camera's flight from one place to another (`trono_balcon`…; a flight only made one way is
//! played backwards for the way back). The frames are decoded on a thread of their own, a few
//! ahead of the one on show (and the next loop's first ones as a loop nears its end).
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;

use crate::ArtBitmap;

const FPS: f32 = 24.0;
/// Frames asked for ahead of the one on show, and kept decoded.
const AHEAD: usize = 4;
const KEEP: usize = 10;

struct Playing {
    clip: String,
    started: Instant,
    looping: bool,
    backwards: bool,
    /// The loop to go on with when a flight ends.
    then: Option<String>,
    /// A loop: its place, and the loop that follows it.
    place: Option<String>,
    next: Option<String>,
}

pub(crate) struct TitleReel {
    clips: HashMap<String, Vec<PathBuf>>,
    /// Each place's loops: the plain one (if any) first, then its variants in name order.
    loops: HashMap<String, Vec<String>>,
    /// How many variants each place has played (whose turn is next).
    turns: HashMap<String, usize>,
    playing: Option<Playing>,
    ask: Sender<(String, usize, PathBuf)>,
    got: Receiver<(String, usize, ArtBitmap)>,
    decoded: Vec<(String, usize, ArtBitmap)>,
    asked: Vec<(String, usize)>,
    /// The frame on show last (kept while the next one is decoding).
    shown: Option<(String, usize)>,
}

/// The place a loop belongs to (`trono#2` → `trono`).
fn place_of(clip: &str) -> &str {
    clip.split('#').next().unwrap_or(clip)
}

impl TitleReel {
    /// The reel under `root` (`None` when there is no clip at all).
    pub(crate) fn open(root: PathBuf) -> Option<Self> {
        let mut clips = HashMap::new();
        for entry in std::fs::read_dir(&root).ok()?.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let mut frames: Vec<PathBuf> = std::fs::read_dir(&dir)
                .map(|d| {
                    d.flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|x| x == "jpg" || x == "png"))
                        .collect()
                })
                .unwrap_or_default();
            if frames.is_empty() {
                continue;
            }
            frames.sort();
            let name = dir.file_name()?.to_string_lossy().into_owned();
            clips.insert(name, frames);
        }
        if clips.is_empty() {
            return None;
        }
        // A loop is a clip without `_` (a flight joins two places with one).
        let mut loops: HashMap<String, Vec<String>> = HashMap::new();
        let mut names: Vec<&String> = clips.keys().filter(|n| !n.contains('_')).collect();
        names.sort_by_key(|n| (n.contains('#'), n.to_string()));
        for name in names {
            loops
                .entry(place_of(name).to_owned())
                .or_default()
                .push(name.clone());
        }
        let (ask, asks) = channel::<(String, usize, PathBuf)>();
        let (send, got) = channel::<(String, usize, ArtBitmap)>();
        std::thread::Builder::new()
            .name("title-reel".into())
            .spawn(move || {
                while let Ok((clip, index, path)) = asks.recv() {
                    let Ok(bytes) = std::fs::read(&path) else {
                        continue;
                    };
                    let Ok(image) = image::load_from_memory(&bytes) else {
                        continue;
                    };
                    let rgba = image.to_rgba8();
                    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                    let bitmap = ArtBitmap {
                        w,
                        h,
                        rgba: rgba.into_raw(),
                    };
                    if send.send((clip, index, bitmap)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        eprintln!("[title] reel: {} clips, loops {:?}", clips.len(), loops);
        Some(Self {
            clips,
            loops,
            turns: HashMap::new(),
            playing: None,
            ask,
            got,
            decoded: Vec::new(),
            asked: Vec::new(),
            shown: None,
        })
    }

    pub(crate) fn has(&self, clip: &str) -> bool {
        self.clips.contains_key(clip)
    }

    /// The loop that follows `clip` at its place: after the plain one, the next variant in turn;
    /// after a variant, the plain one (rest between the things she does).
    fn following(&self, place: &str, clip: &str) -> Option<String> {
        let all = self.loops.get(place)?;
        let plain = all.iter().find(|c| !c.contains('#')).cloned();
        let variants: Vec<&String> = all.iter().filter(|c| c.contains('#')).collect();
        if variants.is_empty() {
            return plain.or_else(|| all.first().cloned());
        }
        if clip.contains('#') {
            if let Some(plain) = plain {
                return Some(plain);
            }
        }
        let turn = self.turns.get(place).copied().unwrap_or(0);
        Some(variants[turn % variants.len()].clone())
    }

    fn start_loop(&mut self, place: &str, clip: String) {
        if clip.contains('#') {
            *self.turns.entry(place.to_owned()).or_default() += 1;
        }
        let next = self.following(place, &clip);
        self.playing = Some(Playing {
            clip,
            started: Instant::now(),
            looping: true,
            backwards: false,
            then: None,
            place: Some(place.to_owned()),
            next,
        });
    }

    /// The place's loop, from wherever it is (nothing on show when it has none: the title shows
    /// the place's picture).
    pub(crate) fn idle(&mut self, place: &str) {
        if self
            .playing
            .as_ref()
            .is_some_and(|p| p.looping && p.place.as_deref() == Some(place))
        {
            return;
        }
        let first = self.loops.get(place).and_then(|all| {
            all.iter()
                .find(|c| !c.contains('#'))
                .or_else(|| all.first())
                .cloned()
        });
        match first {
            Some(clip) => self.start_loop(place, clip),
            None => self.playing = None,
        }
    }

    /// The camera's flight from one place to the other, then the place's loop; false when there
    /// is no flight between them either way (the caller crosses over instead).
    pub(crate) fn fly(&mut self, from: &str, to: &str) -> bool {
        let there = format!("{from}_{to}");
        let back = format!("{to}_{from}");
        let (clip, backwards) = if self.has(&there) {
            (there, false)
        } else if self.has(&back) {
            (back, true)
        } else {
            self.idle(to);
            return false;
        };
        self.playing = Some(Playing {
            clip,
            started: Instant::now(),
            looping: false,
            backwards,
            then: Some(to.to_owned()),
            place: None,
            next: None,
        });
        true
    }

    fn request(&mut self, clip: &str, index: usize) {
        let known = self
            .decoded
            .iter()
            .any(|(c, i, _)| c == clip && *i == index)
            || self.asked.iter().any(|(c, i)| c == clip && *i == index);
        if known {
            return;
        }
        if let Some(path) = self.clips.get(clip).and_then(|f| f.get(index)) {
            if self
                .ask
                .send((clip.to_owned(), index, path.clone()))
                .is_ok()
            {
                self.asked.push((clip.to_owned(), index));
            }
        }
    }

    /// The frame to show now (`None` until one is decoded), asking for the next ones.
    pub(crate) fn frame(&mut self) -> Option<&ArtBitmap> {
        // A loop that has come round hands over to the next one at its place; a flight that has
        // ended, to its place's loop.
        let (clip, index) = loop {
            let p = self.playing.as_ref()?;
            let frames = self.clips.get(&p.clip)?.len();
            let at = (p.started.elapsed().as_secs_f32() * FPS) as usize;
            if at < frames {
                break (
                    p.clip.clone(),
                    if p.backwards { frames - 1 - at } else { at },
                );
            }
            if p.looping {
                let (place, next) = (p.place.clone(), p.next.clone());
                match (place, next) {
                    (Some(place), Some(next)) => self.start_loop(&place, next),
                    _ => {
                        // A lone loop: round again.
                        if let Some(p) = self.playing.as_mut() {
                            p.started = Instant::now();
                        }
                    }
                }
                continue;
            }
            let then = p.then.clone();
            self.playing = None;
            if let Some(place) = then {
                self.idle(&place);
            }
        };
        let (backwards, frames, next) = {
            let p = self.playing.as_ref()?;
            (p.backwards, self.clips.get(&clip)?.len(), p.next.clone())
        };
        // What came back from the thread.
        while let Ok((c, i, bitmap)) = self.got.try_recv() {
            self.asked.retain(|(ac, ai)| !(ac == &c && *ai == i));
            self.decoded.push((c, i, bitmap));
        }
        // Ask for this frame and the next few (into the next loop's first ones at a loop's end).
        for k in 0..AHEAD {
            if backwards {
                if let Some(i) = index.checked_sub(k) {
                    self.request(&clip, i);
                }
            } else if index + k < frames {
                self.request(&clip, index + k);
            } else if let Some(next) = next.clone() {
                self.request(&next, index + k - frames);
            }
        }
        // Keep only what is near: the frames ahead of this one, and the next loop's first ones.
        let shown = self.shown.clone();
        let near = |c: &str, i: usize| {
            (c == clip
                && if backwards {
                    i <= index && index - i < KEEP
                } else {
                    i >= index && i - index < KEEP
                })
                || (next.as_deref() == Some(c) && i < AHEAD)
                || shown.as_ref().is_some_and(|(sc, si)| sc == c && *si == i)
        };
        self.decoded.retain(|(c, i, _)| near(c, *i));
        if self
            .decoded
            .iter()
            .any(|(c, i, _)| c == &clip && *i == index)
        {
            self.shown = Some((clip.clone(), index));
        }
        let (sc, si) = self.shown.clone()?;
        self.decoded
            .iter()
            .find(|(c, i, _)| *c == sc && *i == si)
            .map(|(_, _, b)| b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reel(names: &[&str]) -> TitleReel {
        let (ask, _asks) = channel();
        let (_send, got) = channel();
        let clips: HashMap<String, Vec<PathBuf>> = names
            .iter()
            .map(|n| (n.to_string(), vec![PathBuf::from("x.jpg")]))
            .collect();
        let mut loops: HashMap<String, Vec<String>> = HashMap::new();
        let mut list: Vec<&str> = names.iter().copied().filter(|n| !n.contains('_')).collect();
        list.sort_by_key(|n| (n.contains('#'), n.to_string()));
        for n in list {
            loops
                .entry(place_of(n).to_owned())
                .or_default()
                .push(n.to_owned());
        }
        TitleReel {
            clips,
            loops,
            turns: HashMap::new(),
            playing: None,
            ask,
            got,
            decoded: Vec::new(),
            asked: Vec::new(),
            shown: None,
        }
    }

    #[test]
    fn a_place_rests_between_the_things_she_does_taken_in_turn() {
        let mut r = reel(&[
            "trono",
            "trono#1",
            "trono#2",
            "trono#3",
            "trono_balcon",
            "balcon",
        ]);
        r.idle("trono");
        let mut seen = vec![r.playing.as_ref().unwrap().clip.clone()];
        for _ in 0..6 {
            let p = r.playing.as_ref().unwrap();
            let (place, next) = (p.place.clone().unwrap(), p.next.clone().unwrap());
            r.start_loop(&place, next);
            seen.push(r.playing.as_ref().unwrap().clip.clone());
        }
        assert_eq!(
            seen,
            ["trono", "trono#1", "trono", "trono#2", "trono", "trono#3", "trono"]
        );
        // A place with one loop goes round it; a flight is no loop.
        r.idle("balcon");
        assert_eq!(r.playing.as_ref().unwrap().next.as_deref(), Some("balcon"));
        assert!(!r.loops.contains_key("trono_balcon"));
    }
}
