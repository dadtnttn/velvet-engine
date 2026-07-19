# Wiring inventory — public APIs vs real consumers

Status legend: **wired** · **orphan** · **partial** · **intentional**

| API / surface | Owner crate | Consumer today | Status | Connection owner |
|---------------|-------------|----------------|--------|------------------|
| `.vstory` → `StoryProgram` | `velvet-story-lang` | CLI story, boot, tests | **wired** | `boot`, story_cmd |
| Unified boot `.vstory`/`.vel` | `velvet-story-lang::boot` | `velvet play`, `velvet-runtime` | **wired** | play_cmd, runtime |
| `StoryPlayer` nested/save/host | `velvet-story` | unit + product tests | **wired** | runtime |
| `StoryCommandHost` / `combat.start` | `velvet-action::CombatStoryHost` | action unit test + games | **wired** | story_host.rs |
| `StoryEvent` Music/Sound | product `VnSession` | `music_and_sound_wire_to_product_signals`, CLI HostAudio | **wired** | product ingest |
| `DialogueBridge` → player | `velvet-rpg` | `start_dialogue` + test | **wired** | dialogue_bridge |
| Studio model API | `velvet-story-lang` / editor | CLI `studio-model`, palette `story-outline` | **wired** | commands + CLI |
| `StoryPlugin` | `velvet-story` | windowed play tick, docs | **wired** | plugin + docs |
| OpVs2 from StoryProgram | story-lang | dump-lowered / fallback | **intentional** secondary | pipeline |
| Full Studio visual edit | editor | limited | **intentional** residual | future GUI work |

## Spine (product)

```text
.vstory → story-lang boot → StoryProgram → StoryPlayer / VnSession
                              ├─ CombatStoryHost (action)
                              ├─ StoryEvent → BGM/SFX presentation
                              └─ DialogueBridge.start_dialogue (rpg)
```
