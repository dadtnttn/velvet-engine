//! Stable SplitMix64 streams. Never use DefaultHasher, thread_rng or iteration
//! order of a HashMap in generation. Stream names are part of the file format.
#[derive(Clone, Debug)]
pub struct Rng(u64);
impl Rng {
    pub fn stream(seed: u64, domain: &str, index: u64) -> Self {
        Self(seed ^ hash(domain.as_bytes()) ^ index.wrapping_mul(0x9e3779b97f4a7c15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / 16777216.0
    }
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
    pub fn index(&mut self, count: usize) -> usize {
        (self.next_u64() % count.max(1) as u64) as usize
    }
    pub fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability
    }
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = self.index(i + 1);
            values.swap(i, j);
        }
    }
}
pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x100000001b3)
    })
}
