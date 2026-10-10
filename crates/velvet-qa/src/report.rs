//! What a QA run found: per zone and per incident, written as JSON and as Markdown.
use std::fmt::Write as _;
use std::path::Path;
use velvet_nav::NavMesh;

/// One thing that went wrong in a zone.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Incident {
    /// The zone's name.
    pub zone: String,
    /// What kind of thing: `stuck`, `island`, `no-prompt`, `broken`, `slow-frame` (or a
    /// game's own).
    pub kind: &'static str,
    /// Where (x, z).
    pub at: [f32; 2],
    /// What happened, for a reader.
    pub detail: String,
    /// The screenshot's file name, next to the report.
    pub shot: Option<String>,
}

/// What the walker found in a zone.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ZoneReport {
    /// The zone's name.
    pub zone: String,
    /// All the walkable ground (m²).
    pub walkable_m2: f32,
    /// The ground reachable from the start (m²).
    pub reachable_m2: f32,
    /// Walkable by the collision but outside the building (reaching the zone's edge).
    pub outside_m2: f32,
    /// Walkable ground of a square metre or more nobody can get to.
    pub islands: usize,
    /// Places it set out for.
    pub targets: usize,
    /// Places it got to.
    pub reached: usize,
    /// Places it could not get to though the navmesh had a way.
    pub stuck: usize,
    /// How far it walked (m).
    pub walked_m: f32,
    /// The worst frame (ms).
    pub frame_worst_ms: f32,
    /// The 95th percentile frame (ms).
    pub frame_p95_ms: f32,
    /// The mean frame (ms).
    pub frame_mean_ms: f32,
    /// Every interaction prompt met.
    pub prompts: Vec<String>,
    /// The known interactables (doors, hotspots) walked to.
    pub interactables: usize,
    /// Those that showed a prompt when it stood at them.
    pub interactables_ok: usize,
    /// How long the zone took (s).
    pub seconds: f32,
}

impl ZoneReport {
    /// The line a run prints for the zone (tools that compare runs read it).
    pub fn summary(&self) -> String {
        format!(
            "{}: {}/{} targets, {} stuck, {:.0}/{:.0} m² reachable, {} islands, frames mean {:.1} p95 {:.1} worst {:.1} ms, {} prompts, {}/{} interactables ({:.0} s)",
            self.zone,
            self.reached,
            self.targets,
            self.stuck,
            self.reachable_m2,
            self.walkable_m2,
            self.islands,
            self.frame_mean_ms,
            self.frame_p95_ms,
            self.frame_worst_ms,
            self.prompts.len(),
            self.interactables_ok,
            self.interactables,
            self.seconds
        )
    }
}

/// Writes the run's report into `out`: `qa-report.json` (everything) and `qa-report.md` (a table
/// of the zones, the incidents with their screenshots, the prompts met; in Spanish).
pub fn write_report(
    out: &Path,
    reports: &[ZoneReport],
    incidents: &[Incident],
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(
        &serde_json::json!({ "zones": reports, "incidents": incidents }),
    )
    .map_err(std::io::Error::other)?;
    std::fs::write(out.join("qa-report.json"), json)?;
    let mut md = String::new();
    let _ = writeln!(md, "# Recorrido de QA\n");
    let _ = writeln!(md, "| zona | objetivos | atascos | alcanzable | islas | frame medio | p95 | peor | avisos F | interactuables |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|");
    for r in reports {
        let _ = writeln!(
            md,
            "| {} | {}/{} | {} | {:.0}/{:.0} m² | {} | {:.1} ms | {:.1} ms | {:.1} ms | {} | {}/{} |",
            r.zone,
            r.reached,
            r.targets,
            r.stuck,
            r.reachable_m2,
            r.walkable_m2,
            r.islands,
            r.frame_mean_ms,
            r.frame_p95_ms,
            r.frame_worst_ms,
            r.prompts.len(),
            r.interactables_ok,
            r.interactables
        );
    }
    let _ = writeln!(md, "\n## Incidencias\n");
    for i in incidents {
        let _ = writeln!(
            md,
            "- **{}** · {} · ({:.1}, {:.1}) · {}{}",
            i.zone,
            i.kind,
            i.at[0],
            i.at[1],
            i.detail,
            i.shot
                .as_ref()
                .map(|s| format!(" · ![]({s})"))
                .unwrap_or_default()
        );
    }
    let _ = writeln!(md, "\n## Avisos F encontrados\n");
    for r in reports {
        let _ = writeln!(md, "- **{}**: {}", r.zone, r.prompts.join(", "));
    }
    std::fs::write(out.join("qa-report.md"), md)
}

/// A picture of a zone's navmesh, `scale` pixels a cell, as (width, height, RGBA8): the reachable
/// ground green, the rest red, the start a yellow dot and each of `marks` (where the walker got
/// stuck) a magenta one.
pub fn zone_map(
    nav: &NavMesh,
    reach: &[bool],
    start: [f32; 2],
    marks: &[[f32; 2]],
    scale: u32,
) -> (u32, u32, Vec<u8>) {
    let (w, h, mut px) = nav.picture(scale, |r| {
        if reach[r as usize] {
            [92, 168, 104]
        } else {
            [196, 72, 72]
        }
    });
    let mut dot = |x: f32, z: f32, c: [u8; 3], radius: i64| {
        let (cx, cy) = nav.pixel_of(x, z, scale);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let (px_x, px_y) = (cx + dx, cy + dy);
                if dx * dx + dy * dy <= radius * radius
                    && px_x >= 0
                    && px_y >= 0
                    && (px_x as u32) < w
                    && (px_y as u32) < h
                {
                    let k = ((px_y as u32 * w + px_x as u32) * 4) as usize;
                    px[k..k + 3].copy_from_slice(&c);
                }
            }
        }
    };
    dot(start[0], start[1], [250, 220, 90], 6);
    for at in marks {
        dot(at[0], at[1], [250, 60, 200], 5);
    }
    (w, h, px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_says_what_was_found() {
        let dir = std::env::temp_dir().join(format!("velvet-qa-report-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let zone = ZoneReport {
            zone: "patio".into(),
            targets: 3,
            reached: 2,
            stuck: 1,
            walkable_m2: 40.0,
            reachable_m2: 38.4,
            prompts: vec!["Abrir".into()],
            interactables: 1,
            interactables_ok: 1,
            ..Default::default()
        };
        assert_eq!(
            zone.summary(),
            "patio: 2/3 targets, 1 stuck, 38/40 m² reachable, 0 islands, frames mean 0.0 p95 0.0 worst 0.0 ms, 1 prompts, 1/1 interactables (0 s)"
        );
        let incident = Incident {
            zone: "patio".into(),
            kind: "stuck",
            at: [1.0, 2.0],
            detail: "could not get to t1".into(),
            shot: Some("patio-stuck-0.png".into()),
        };
        write_report(&dir, &[zone], &[incident]).unwrap();
        let md = std::fs::read_to_string(dir.join("qa-report.md")).unwrap();
        assert!(md.contains("| patio | 2/3 | 1 | 38/40 m² |"), "{md}");
        assert!(
            md.contains(
                "- **patio** · stuck · (1.0, 2.0) · could not get to t1 · ![](patio-stuck-0.png)"
            ),
            "{md}"
        );
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("qa-report.json")).unwrap())
                .unwrap();
        assert_eq!(json["zones"][0]["reached"], 2);
        assert_eq!(json["incidents"][0]["kind"], "stuck");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_map_marks_the_start_and_the_snags() {
        let nav = NavMesh::build([0.0, 0.0, 4.0, 4.0], 0.25, 10.0, |x, _| {
            (x < 3.0).then_some(0.0)
        });
        let reach = vec![true; nav.rects.len()];
        let (w, h, px) = zone_map(&nav, &reach, [1.0, 1.0], &[[2.0, 3.0]], 4);
        assert_eq!(px.len(), (w * h * 4) as usize);
        let at = |x: f32, z: f32| {
            let (cx, cy) = nav.pixel_of(x, z, 4);
            let k = ((cy as u32 * w + cx as u32) * 4) as usize;
            [px[k], px[k + 1], px[k + 2]]
        };
        assert_eq!(at(1.0, 1.0), [250, 220, 90]);
        assert_eq!(at(2.0, 3.0), [250, 60, 200]);
        assert_eq!(at(2.5, 0.5), [92, 168, 104]);
    }
}
