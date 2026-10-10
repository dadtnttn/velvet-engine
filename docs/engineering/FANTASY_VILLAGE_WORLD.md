# Fantasy Village world

## Source and conversion

The source supplied for this world was the POLY Fantasy Village Blender pack.
The ZIP found on the development machine is:

`C:\Users\usuario\Downloads\POLY_FantasyVillage_Blender.zip`

It contains `POLY_FantasyVillage_Blender.blend` and no external texture files
(the texture is packed in the blend). The scene is a showcase with multiple
clusters. The runtime uses only the lower settlement cluster; castle and terrain
showcase clusters are intentionally excluded.

Regenerate the runtime asset with the Blender helper (the underscores in the
downloaded filename are the escaped `POLY\_FantasyVillage\_Blender.zip` name
from the request):

```powershell
$zip = "C:\Users\usuario\Downloads\POLY_FantasyVillage_Blender.zip"
$source = Join-Path $env:TEMP "fantasy_village_source"
Expand-Archive -LiteralPath $zip -DestinationPath $source -Force
$blend = Get-ChildItem -LiteralPath $source -Filter *.blend -Recurse | Select-Object -First 1
& "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe" `
  --background $blend.FullName `
  --python tools\fantasy_village_to_glb.py -- `
  --output demos\demon-queen\data\assets\3d\fantasy_village\fantasy_village.glb
```

The helper selects mesh objects whose authored object origin is below `Y=-40`,
adds the six `SM_Raod_01/02/03` path slabs, moves that path group just inside
the village entrance, and exports one centred GLB. It never saves over the
Blender source. The generated GLB is a local runtime asset under the ignored
`demos/` tree, so a fresh checkout must run the conversion (or copy the
generated asset) before launching the 3D demo.

## Runtime integration

`demos/demon-queen/src/exploration3d.rs` loads
`data/assets/3d/fantasy_village/fantasy_village.glb` and places it at the end of
the outdoor trail:

- gate/entry anchor: `VILLAGE_GATE_Z = 20.05`;
- village centre: same trail X, world Z `32.0`;
- imported scale: `0.30`;
- playable village bounds: ±`47` world units;
- F near the gate enters the village; F near the return marker leaves it;
- N continues to switch castle and outdoor terrain modes.

The village mode hides the forest preview and mage, draws the imported houses
and path slabs, and keeps the terrain gate marker visible for a reversible
transition. Static collision boxes are the footprint approximation for the
exported buildings, carts and piers; the road slabs stay walkable. If the source
selection, scale, or placement changes, update
`fantasy_village_position_blocked` together with the GLB.

`demos/demon-queen/tools/build_portable.ps1` copies the `fantasy_village` pack
into portable builds alongside the forest and castle packs. The pack is
environment-only for now; NPCs and village-specific interactions can be added
without changing the world transition.

## Verification

From the repository root:

```powershell
cargo check -p demon-queen
cargo build -p demon-queen --bin demon-queen-3d --features console
```

On first village entry, the 3D log should include `Fantasy Village loaded` and the resolved
GLB path. If the asset is missing, the game stays launchable and logs a clear
`Fantasy Village unavailable` message instead of crashing.

### Movement/device-loss regression

On the development RTX 5080 Laptop (driver 610.74), animated movement reproduced
an intermittent Vulkan `Surface::present: Parent device is lost` panic even at
MSAA 1x. An idle-window check does not cover this failure. The shared window
display now defaults to DX12 on Windows; `WGPU_BACKEND` remains an explicit
diagnostic override. Adapter/backend and device-loss reasons are logged.

The renderer batches aligned object uniforms into one reusable CPU buffer and
one GPU upload per frame, instead of allocating an upload for each draw. This
also reduces the cost of dense forest frames while characters animate.

The standalone preview provides a bounded walking/running regression run:

```powershell
Copy-Item "$env:APPDATA\VelvetEngine\DemonQueen\saves\exploration-3d.json" temp\movement-test.json
cargo run -p demon-queen --bin demon-queen-3d --features console -- --movement-smoke --session temp\movement-test.json
```

It exercises four movement directions, walking then sprinting, through the
normal collision, animation, footprint and rendering paths for 24 simulation
seconds. Expect `[movement-smoke] PASS` and exit code 0. The explicit test save
is mandatory: use a disposable copy, never the player's real checkpoint.
Repeat with `nature_mode: true, village_entered: true` and an entrance position
near `x: -1.32, z: 21.5` in that test save to cover the imported village.
