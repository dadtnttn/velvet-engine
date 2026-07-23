//! Spatial hash for free particles and agents — O(1) neighbor queries.

use crate::particles::FreeParticle;

/// Cell key in hash grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashKey {
    /// X bucket.
    pub x: i32,
    /// Y bucket.
    pub y: i32,
}

/// Spatial hash map from bucket → particle indices.
#[derive(Debug, Clone, Default)]
pub struct SpatialHash {
    /// Bucket size in world units.
    pub cell_size: f32,
    /// Map.
    map: std::collections::HashMap<HashKey, Vec<usize>>,
}

impl SpatialHash {
    /// Create with cell size.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(0.5),
            map: std::collections::HashMap::new(),
        }
    }

    fn key(&self, x: f32, y: f32) -> HashKey {
        HashKey {
            x: (x / self.cell_size).floor() as i32,
            y: (y / self.cell_size).floor() as i32,
        }
    }

    /// Clear.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Rebuild from particles.
    pub fn rebuild(&mut self, particles: &[FreeParticle]) {
        self.map.clear();
        for (i, p) in particles.iter().enumerate() {
            if !p.alive {
                continue;
            }
            let k = self.key(p.x, p.y);
            self.map.entry(k).or_default().push(i);
        }
    }

    /// Insert one index.
    pub fn insert(&mut self, x: f32, y: f32, idx: usize) {
        let k = self.key(x, y);
        self.map.entry(k).or_default().push(idx);
    }

    /// Query indices near point within radius (bucket neighborhood).
    pub fn query(&self, x: f32, y: f32, radius: f32) -> Vec<usize> {
        let r_cells = (radius / self.cell_size).ceil() as i32 + 1;
        let cx = (x / self.cell_size).floor() as i32;
        let cy = (y / self.cell_size).floor() as i32;
        let mut out = Vec::new();
        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                let k = HashKey {
                    x: cx + dx,
                    y: cy + dy,
                };
                if let Some(v) = self.map.get(&k) {
                    out.extend_from_slice(v);
                }
            }
        }
        out
    }

    /// Count buckets.
    pub fn bucket_count(&self) -> usize {
        self.map.len()
    }

    /// Total indexed entries.
    pub fn entry_count(&self) -> usize {
        self.map.values().map(|v| v.len()).sum()
    }
}

/// Pairwise soft separation for overlapping free particles (prevents stacking glitches).
pub fn separate_particles(
    particles: &mut [FreeParticle],
    hash: &SpatialHash,
    min_dist: f32,
) -> u32 {
    let min_dist2 = min_dist * min_dist;
    let mut fixes = 0u32;
    let snapshot: Vec<(usize, f32, f32)> = particles
        .iter()
        .enumerate()
        .filter(|(_, p)| p.alive)
        .map(|(i, p)| (i, p.x, p.y))
        .collect();
    for &(i, x, y) in &snapshot {
        let neighbors = hash.query(x, y, min_dist * 2.0);
        for &j in &neighbors {
            if j <= i {
                continue;
            }
            if !particles[j].alive {
                continue;
            }
            let dx = particles[j].x - particles[i].x;
            let dy = particles[j].y - particles[i].y;
            let d2 = dx * dx + dy * dy;
            if d2 < 1e-8 || d2 >= min_dist2 {
                continue;
            }
            let d = d2.sqrt();
            let push = (min_dist - d) * 0.5;
            let nx = dx / d;
            let ny = dy / d;
            particles[i].x -= nx * push;
            particles[i].y -= ny * push;
            particles[j].x += nx * push;
            particles[j].y += ny * push;
            fixes += 1;
        }
    }
    fixes
}

/// Average density estimate: particles per bucket.
pub fn average_bucket_load(hash: &SpatialHash) -> f32 {
    if hash.map.is_empty() {
        return 0.0;
    }
    hash.entry_count() as f32 / hash.bucket_count() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::MaterialId;
    use crate::particles::FreeParticle;

    #[test]
    fn hash_query_finds_neighbors() {
        let mut parts = vec![
            FreeParticle::new(1, 0.0, 0.0, MaterialId::AIR),
            FreeParticle::new(2, 0.5, 0.2, MaterialId::AIR),
            FreeParticle::new(3, 50.0, 50.0, MaterialId::AIR),
        ];
        for p in &mut parts {
            p.alive = true;
        }
        let mut h = SpatialHash::new(2.0);
        h.rebuild(&parts);
        let q = h.query(0.0, 0.0, 2.0);
        assert_eq!(q.len(), 2, "query={q:?}");
        assert!(q.contains(&0) && q.contains(&1));
        assert!(!q.contains(&2), "distant particle leaked into query: {q:?}");
        let fixes = separate_particles(&mut parts, &h, 1.0);
        assert!(fixes >= 1);
        let dx = parts[0].x - parts[1].x;
        let dy = parts[0].y - parts[1].y;
        assert!((dx * dx + dy * dy).sqrt() >= 1.0 - 1e-4);
    }
}
