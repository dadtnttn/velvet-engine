# Velvet Image & Crypto tools (VS2 surface)

## Image (`velvet-image`)

Tools for **types** and **compression** (encode quality). Not encryption.

| API | Role |
|-----|------|
| `probe(bytes)` | kind, width, height, alpha |
| `decode_rgba(bytes)` | PNG/JPEG → RGBA |
| `encode(rgba, ImageEncode)` | PNG or JPEG (quality 1–100) |
| `rasterize_simple_svg(svg, w, h)` | Minimal path fill for icons |
| `build_svg_document(...)` | Build SVG XML |

CLI:

```bash
cargo run -p velvet-cli -- image info path.png
cargo run -p velvet-cli -- image convert in.png out.jpg --quality 85
```

VS2 natives (stdlib / VM): math helpers + `hash_sha256` / codec (see crypto). Image **file** I/O is CLI/host; pure buffers via Rust tools.

## Crypto (`velvet-crypto`)

Game tools for VS2 — **not** FIPS. Caps: 1 MiB hash input, 64 KiB random.

| API | Role |
|-----|------|
| `hash_sha256` / hex | checksums, seeds |
| `hmac_sha256` | keyed tokens |
| `random_bytes(n)` | CSPRNG |
| `constant_time_eq` | secret compare |
| `hex_encode` / `hex_decode` | display |
| `base64_encode` / `base64_decode` | payloads |

VS2 natives: `hash_sha256`, `hex_encode`, `base64_encode`, plus math `sin`/`cos`/`sqrt`/`pow`/`lerp`.

## SVG in `.vcss`

```css
@svg badge {
  viewBox: 0 0 64 64;
  fill: #ebc878;
  path: "M0,32 L32,0 L64,32 L32,64 Z";
}
.chip { background-image: svg(badge); width: 64; height: 64; }
.panel { background-image: url("ui/bg.png"); }
```

Compression of exported PNG from SVG: use **VS2/CLI image.encode**, not CSS.
