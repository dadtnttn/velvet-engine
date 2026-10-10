//! Human-readable reproduction artifacts. SVG is drawn from the blueprint,
//! never a decorative substitute for actual geometry.
use super::*;
pub fn svg(map: &Blueprint) -> String {
    let min_x = map
        .rooms
        .iter()
        .map(|r| r.center.x - r.half[0])
        .fold(f32::INFINITY, f32::min)
        - 8.0;
    let min_z = map
        .rooms
        .iter()
        .map(|r| r.center.z - r.half[1])
        .fold(f32::INFINITY, f32::min)
        - 8.0;
    let max_x = map
        .rooms
        .iter()
        .map(|r| r.center.x + r.half[0])
        .fold(f32::NEG_INFINITY, f32::max)
        + 8.0;
    let max_z = map
        .rooms
        .iter()
        .map(|r| r.center.z + r.half[1])
        .fold(f32::NEG_INFINITY, f32::max)
        + 8.0;
    let scale = (1050.0 / (max_x - min_x)).min(710.0 / (max_z - min_z));
    let x = |v: f32| 30.0 + (v - min_x) * scale;
    let y = |v: f32| 100.0 + (v - min_z) * scale;
    let mut s=format!("<svg xmlns='http://www.w3.org/2000/svg' width='1400' height='900' viewBox='0 0 1400 900'><rect width='1400' height='900' fill='#101f25'/><g font-family='Segoe UI, sans-serif' fill='#f1e3c2'><text x='34' y='43' font-size='28'>DUNGEON / seed {} / v{}</text><text x='34' y='72' font-size='15'>{} rooms · {} loops · {} vertical links · hash {:016x}</text></g>",map.seed,map.version,map.stats.rooms,map.stats.loops,map.stats.vertical_connections,map.structural_hash());
    for c in &map.connections {
        let color = if c.critical {
            "#e8bf74"
        } else if c.kind == ConnectionKind::Secret {
            "#cf83d5"
        } else {
            "#62a5a5"
        };
        s += &format!(
            "<path d='M {} {} L {} {}' stroke='{color}' stroke-width='{}'/>",
            x(c.start.x),
            y(c.start.z),
            x(c.end.x),
            y(c.end.z),
            c.width * scale
        );
    }
    for r in &map.rooms {
        let colors = ["#355556", "#266579", "#72423e"];
        let points = r
            .outline
            .iter()
            .map(|p| format!("{},{}", x(r.center.x + p[0]), y(r.center.z + p[1])))
            .collect::<Vec<_>>()
            .join(" ");
        s+=&format!("<polygon points='{points}' fill='{}' fill-opacity='.9' stroke='#d0ad70' stroke-width='1.2'/><text x='{}' y='{}' fill='#fff0d2' font-family='Segoe UI' font-size='10' text-anchor='middle'>{}: {}</text><text x='{}' y='{}' fill='#c7d0ce' font-family='Segoe UI' font-size='9' text-anchor='middle'>y {:+.1}m</text>",colors[r.biome.min(2)],x(r.center.x),y(r.center.z)-2.0,r.id,r.role.label(),x(r.center.x),y(r.center.z)+11.0,r.center.y);
        for p in &r.sockets {
            s += &format!(
                "<circle cx='{}' cy='{}' r='2.5' fill='#fff4cb'/>",
                x(p.position.x),
                y(p.position.z)
            );
        }
    }
    for spawn in &map.spawns {
        s += &format!(
            "<circle cx='{}' cy='{}' r='3' fill='{}'/>",
            x(spawn.position.x),
            y(spawn.position.z),
            if spawn.health > 0.0 {
                "#ff8181"
            } else {
                "#7be3b0"
            }
        );
    }
    s+=&format!("<g fill='#e6d5b3' font-family='Segoe UI' font-size='15'><text x='1120' y='140'>Critical path: {}</text><text x='1120' y='172'>Branches: {}</text><text x='1120' y='204'>Stacked rooms: {}</text><text x='1120' y='236'>Repairs: {}</text><text x='1120' y='268'>Rejected: {} / {}</text><text x='1120' y='300'>Generation: {:.2} ms</text><text x='1120' y='332'>Volume: {:.0} m³</text><text x='1120' y='380'>Gold: critical route</text><text x='1120' y='408'>Teal: optional route</text><text x='1120' y='436'>Violet: secret</text><text x='1120' y='464'>Red: encounters</text><text x='1120' y='492'>Mint: interactions</text></g></svg>",map.stats.critical_length,map.stats.branches,map.stats.stacked_rooms,map.stats.local_repairs,map.stats.rejected_attempts,map.stats.attempts,map.stats.generation_ms,map.stats.volume);
    s
}
