# velvet-cellular — fully functional Noita-like author core

## Purpose

Creation-focused cellular simulation so authors build falling-sand / pixel-material
games **with free particles that couple back to the grid**, spells, agents, and
efficient hot-chunk stepping. Not a commercial Noita clone.

## Creator surface (shipped)

| System | Capability |
|--------|------------|
| **Grid world** | Chunked 64², materials with phase/density/fire/blood/dissolve/explosion |
| **Free particles** | Burst, emitter, settle→`ConvertToCell` / dig / heat |
| **Particle presets** | Blood, sparks, dig debris, steam, poison, … |
| **Spells** | 100+ data recipes (`spark_bolt`, `spell_000`, …) |
| **Agents** | Dig/place/cast player body vs solids |
| **Enemies** | Blueprints + AI + death gore |
| **Brush** | Shapes/modes including Bleed/Dig/Ignite |
| **Procgen** | Caves, arena, platforms, scatter |
| **Forces** | Wind, gravity well, heat/cold zones |
| **Efficiency** | Hot-chunk step + timed perf test (2 chunks, 30 steps) |
| **Materials** | Builtins + compact data catalog (100+ unique keys, table-driven) |
| **Fluid / electricity / growth** | Liquid blobs, conductive arcs, vine/moss CA |
| **Wands / combat** | Spell combos, hitscan, melee splash |
| **Layers / biomes / replay** | Multi-layer stack, biome stamps, deterministic replay |

## Non-claimed (game content)

Full wand meta-progression, multi-hour biomes/campaign, Steam release art parity.

## Entry points

```bash
cargo test -p velvet-cellular
cargo run -p cellular-lab --release
velvet-studio cellular --preset Sand --enemy --steps 60
```

## API sketch

```rust
let mut s = CellularSession::with_builtins(WorldConfig::default());
s.particle_burst(0.0, 20.0, "sand", 32);
s.cast_spell("spark_bolt", 2.0, 15.0);
s.spawn_agent(0.0, 10.0);
s.step_n(60);
let buf = s.render(-64, -16, 128, 96);
```
