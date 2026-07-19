# Wiring inventory — public APIs vs real consumers

Status legend: **wired** · **orphan** · **partial** · **intentional**

| API / surface | Owner crate | Consumer today | Status | Owner of connection |
|---------------|-------------|----------------|--------|---------------------|
| `.vstory` → `StoryProgram` | `velvet-story-lang` | `velvet story run/check`, tests | **wired** | CLI story_cmd |
| `.vel` story → `StoryProgram` | `velvet-story::load` | `velvet play`, `open_session_from_file` | **wired** (legacy path) | CLI play_cmd |
| Unified boot `.vstory` **and** `.vel` | — | play used only `.vel` loader | **partial** → fix PR1 | CLI/runtime boot |
| `StoryPlayer` / nested save / host wait | `velvet-story` | unit tests, story run | **wired** | story runtime |
| `StoryCommandHost` / `combat.start` | `velvet-story` | unit tests only | **orphan** (game host) | demos / action+rpg host |
| `StoryEvent` → BGM/SFX | `velvet-story` product | CLI HostAudio (BGM), sfx_queue fields | **partial** | play_cmd + product |
| `DialogueBridge` | `velvet-rpg` | map only; no StoryPlayer start | **orphan** | integration / hybrid demo |
| `story_studio_model*` | `velvet-editor` | lib test only | **orphan** (GUI) | editor panel / CLI studio-model already exists |
| `velvet story studio-model` | CLI | command exists | **wired** | story_cmd |
| `StoryPlugin` | `velvet-story` | windowed tick smoke | **partial** | demos |
| `velvet-runtime` bin | runtime | 1-frame app smoke | **orphan** (no story) | runtime + boot |
| VS2 script check/run | script crates | `velvet script *` | **wired** | CLI (logic layer, not VN IR) |
| OpVs2 from StoryProgram | story-lang | dump-lowered / fallback | **intentional** secondary | pipeline |

Updated as wiring PRs land. Do not invent second narrative VMs.
