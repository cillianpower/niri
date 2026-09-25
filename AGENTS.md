# AGENTS.md — niri fork

## Latest upstream sync — 2026-09-25

`sync-focus-ring-anim` merged `upstream/main` at `e3c52255` (24 new commits) without conflicts.
The previous sync is recorded below. This branch remains a fork work branch; upstream's
`CONTRIBUTING.md` asks PR authors to rebase and keep feature commits focused. The new PR
template explicitly disallows LLM-written PR code and descriptions, so this branch must
not be presented as a ready-to-submit upstream PR without human authorship and review.

The focus transitions now start from layout refresh, before rendering. Focus-ring selection
and shadow activation have separate transition state: this preserves the selected ring on
inactive monitors and lets the shadow use its inactive color. Both use the same opt-in
animation config. A deselected ring remains rendered and keeps its prior active/inactive
color until its fade-out finishes. The transition helper has a unit test for initialization,
reversal, completion, and `off`. `cargo test --workspace --locked` passed (207 niri tests,
19 config tests, wiki parsing, IPC, and a doc test), including the multi-monitor
selection regression test. Clippy, build, and
nightly format checks passed. The default and throwaway nested configs validated.
A nested winit compositor ran with two Ghostty windows; IPC focus switching and
screenshot captures showed both rings changing alpha smoothly over a one-second
linear fade. The nested config was then restored to 300 ms EaseOutQuad. This
verifies rendering on one winit output. Multi-monitor behavior needs hands-on testing.

## Latest upstream sync — 2026-09-17

On `sync-focus-ring-anim`, existing uncommitted shadow-fade/scheduling work was preserved
in `841a9844`, then `upstream/main` at `b735805d` was merged without conflicts in
`62f8e0ac` (53 upstream commits). No compatibility fixes were needed.
`cargo check --locked`, `cargo build --locked`, `cargo test --locked` (199 passed),
and `cargo clippy --all-targets --locked` passed. Existing local config validation
also passed. Built binary: `target/debug/niri`; nothing installed or restarted.
Visual focus-ring/shadow behavior remains unverified after this sync (headless session).
Older branch, installed-binary, and test-count references below describe prior work.

Fork: `cillianpower/niri`, branch `focus-ring-anim` from tag `v26.04` (matches Fedora 44 `/usr/bin/niri`). Upstream: `niri-wm/niri`. Niri source checked out at `/home/user/Development/niri`.

> **Materials system** is config-only and does not belong in this fork. Design docs and pipeline have moved to the dotfiles repo at `~/Development/dotfiles/docs/niri-materials/` (`README.md`, `materials-design.md`, `architecture.md`, `niri-reference.md`). Implementation is an external orchestration script that emits `~/.config/niri/materials.kdl` + `gsettings`/terminal fan-out — no niri source changes. See that location as the source of truth; do not add materials logic here unless explicitly asked to implement a fork extension (tint/per-output/per-window blur).

## Scope — single active body of work in this repo

| # | Body of work | Status | Docs |
| --- | --- | --- | --- |
| **Focus ring alpha fade + shadow cross-fade** — animate focus ring alpha and shadow color on focus change | **Implemented & dogfooded** on this branch. Off by default (opt-in). | This file + `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md`, `HANDOVER.md` |

This repo is only for work that extends niri itself.

---

## Focus ring alpha fade + shadow cross-fade (dogfooded)

### Status

Implemented, tested, working on branch `focus-ring-anim`. Builds to `/usr/bin/niri` (release + line tables, ~154MB); original Fedora build backed up at `/usr/bin/niri.backup`. Restart via `niri msg action quit` or `systemctl --user restart niri.service`. See `HANDOVER.md:20-29`.

### What it does

Focus ring previously snapped. Now fades alpha `0 ↔ max-opacity` when selection changes; shadow cross-fades between `color` and `inactive_color` when activation changes. The two transitions use the same `animations.focus-ring` config and start together when a selected window gains focus on the active monitor. The feature is **off by default**.

Opt-in config:

```kdl
animations {
    focus-ring {
        duration-ms 300
        curve "ease-out-quad" // or linear, ease-out-cubic, ease-out-expo, cubic-bezier, spring
    }
}
```
`focus-ring { off }` restores instant. See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:56-66` for default-off `Default` impl note.

### Files changed

1. **`niri-config/src/animations.rs:14-45`** — new `FocusRingAnim(pub Animation)` newtype over `Animation`, `Default`, `knuffel::Decode`, field `Animations.focus_ring` + `AnimationsPart` + `merge_clone!`. Pattern follows `ScreenshotUiOpenAnim` etc. Shadow reuses same config (no separate key).
2. **`src/layout/tile.rs`** — `FocusTransition` tracks an initial target and optional animation. `Tile::update_focus()` runs during `Layout::refresh()` via scrolling/floating spaces, before render. Separate ring and shadow transitions preserve distinct selection and activation semantics. Interruption starts from the current progress; `Tile::advance_animations()` clears completed animations; `Tile::are_transitions_ongoing()` keeps frames scheduled.
3. **`resources/default-config.kdl:93-99`** — commented-out example.
4. **`niri-config/src/lib.rs`** — insta snapshot updated.
5. **`niri-config` `Color::mix`** + `Shadow::update_render_elements(focus_progress: f64)` change (`FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:393-410`). Non-focus callers (`mapped.rs`, `workspace.rs`) pass `1.`.

Architecture diagram `AGENTS.md:109-140` (pre-shadow) still applies, with `focus_progress` driving both ring and shadow.

### Edge cases handled

`off` = early-out (no `Animation`, steady-state alpha); first refresh records state without animating (no startup flash); interrupted transitions start from current progress; fullscreen/maximize hidden via `expanded_progress`; config merge via `merge_clone!`. Ring selection remains visible on inactive monitors. Shadow animation is skipped when shadows are disabled.

### Visual verification

The prior render-time transition creation has been replaced with refresh-time creation.
Check smoothness and multi-monitor selection in the nested compositor; static tests do not
prove animation timing on a real display.

### Testing (nested, no session pollution)

Build without installing, run as nested winit compositor inside real session (auto-selected when `WAYLAND_DISPLAY`/`DISPLAY` set and not `--session`):

```bash
cargo build            # or --release
./target/debug/niri --config /tmp/test-niri.kdl  # throwaway config with animations { focus-ring { ... } }
./target/debug/niri validate --config /tmp/test-niri.kdl
```

See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:120-271` for full workflow, throwaway config (`/tmp/niri-focus-ring-test.kdl`), variants (spring, slow 1000ms rapid Alt+Tab), and winit limitations. Terminal on this machine is `ghostty` (not `alacritty`).

### Upstream PR strategy

Keep any upstream submission focused and rebased onto current `upstream/main`. The fork
currently has ring and shadow animation coupled to one config, while the historical PR
plan below proposed ring only; settle the submission scope before preparing commits.
The wiki now documents the option with a `Since: next release` tag.

---

## Working conventions for agents

- **Build/test:** `cargo test` (195 pass expected), `cargo clippy --all-targets`, `cargo build` (never `cargo install` over system niri). Validate KDL with `./target/debug/niri validate --config <file>`.
- **Config parsing:** new `Animation` newtypes follow `knuffel::Decode` newtype pattern + `merge_clone!` triple wiring.
- **Do not touch** `~/.config/niri/config.kdl` or live session unless asked; use `/tmp/*.kdl` + nested winit.
- **Docs sync:** when changing this fork, update `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md` + this file; materials docs live in `~/Development/dotfiles/docs/niri-materials/` — edit there, not here.
