# AGENTS.md — niri fork

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

Focus ring previously snapped. Now fades alpha `0 ↔ max-opacity` on focus change; shadow cross-fades between `color` and `inactive_color` in lockstep, driven by a single shared `focus_progress` animation. Gated by `animations.focus-ring` — **off by default**, opt-in (`FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:9-30`).

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
2. **`src/layout/tile.rs`** — fields `focus_ring_alpha_anim: Option<Animation>` (now `focus_progress_anim`), `prev_focus_ring_is_active: bool`, `focus_ring_initialized: bool`. Transition detection in `Tile::update_render_elements()` creates `Animation::new(clock, current, target, 0, config)`, reads `clamped_value()` as `focus_progress`, computes `ring_alpha = progress * max_opacity` and `shadow_color = color.mix(&inactive_color, 1 - progress)` via `Color::mix` (linear RGBA, in `niri-config`). Cleanup in `Tile::advance_animations()` on `is_done()`. Scheduler fix: registered in `Tile::are_transitions_ongoing()` so frames keep scheduling (`FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:382-386`).
3. **`resources/default-config.kdl:93-99`** — commented-out example.
4. **`niri-config/src/lib.rs`** — insta snapshot updated.
5. **`niri-config` `Color::mix`** + `Shadow::update_render_elements(focus_progress: f64)` change (`FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:393-410`). Non-focus callers (`mapped.rs`, `workspace.rs`) pass `1.`.

Architecture diagram `AGENTS.md:109-140` (pre-shadow) still applies, with `focus_progress` driving both ring and shadow.

### Edge cases handled

`off` = early-out (no `Animation`, steady-state alpha, zero per-frame cost); first frame `focus_ring_initialized` records without animating (no startup flash); interrupted transitions start from current alpha; fullscreen/maximize hidden via `expanded_progress`; config merge via `merge_clone!`. See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:102-110`.

### Known issue — needs expert

Intermittent two-step snap: animation creation in `update_render_elements()` (render time) can skip a frame (VRR idle/no damage) and jump. Correct home is `Layout::refresh()`/`advance_animations()` where focus event originates. Requires transition detection that doesn't misfire on overview/interactive-move/monitor changes and needs clock context. Documented in `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:318-347` with suggested direction. Scheduler fix (`are_transitions_ongoing`) mitigates but architectural move remains for upstream.

### Testing (nested, no session pollution)

Build without installing, run as nested winit compositor inside real session (auto-selected when `WAYLAND_DISPLAY`/`DISPLAY` set and not `--session`):

```bash
cargo build            # or --release
./target/debug/niri --config /tmp/test-niri.kdl  # throwaway config with animations { focus-ring { ... } }
./target/debug/niri validate --config /tmp/test-niri.kdl
```

See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:120-271` for full workflow, throwaway config (`/tmp/niri-focus-ring-test.kdl`), variants (spring, slow 1000ms rapid Alt+Tab), and winit limitations. Terminal on this machine is `ghostty` (not `alacritty`).

### Upstream PR strategy

Single feature, off by default, 3 commits (`animations: add focus-ring fade config type (off by default)`, `layout/tile: animate focus ring alpha on focus change`, `docs: document ...`), wiki `Configuration:-Animations.md` with `Since:` tag, snapshot via `cargo insta`. See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:273-288`.

---

## Working conventions for agents

- **Build/test:** `cargo test` (195 pass expected), `cargo clippy --all-targets`, `cargo build` (never `cargo install` over system niri). Validate KDL with `./target/debug/niri validate --config <file>`.
- **Config parsing:** new `Animation` newtypes follow `knuffel::Decode` newtype pattern + `merge_clone!` triple wiring.
- **Do not touch** `~/.config/niri/config.kdl` or live session unless asked; use `/tmp/*.kdl` + nested winit.
- **Docs sync:** when changing this fork, update `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md` + this file; materials docs live in `~/Development/dotfiles/docs/niri-materials/` — edit there, not here.
