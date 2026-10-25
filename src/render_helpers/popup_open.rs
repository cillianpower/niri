//! Open animation for xdg popups: a fade in with a few pixels of travel away from the anchor.
//!
//! The state lives in the popup surface's data map so that rendering, which only has the surface,
//! can read it. The real popup geometry never moves, so input always goes to its final position.

use std::cell::RefCell;

use smithay::backend::renderer::utils::with_renderer_surface_state;
use smithay::desktop::PopupKind;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource as _;
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::with_states;

use crate::animation::{Animation, Clock};

/// How far a popup travels as it opens, in logical pixels.
const TRAVEL: f64 = 6.;

#[derive(Default)]
enum State {
    /// The popup hasn't shown a buffer yet.
    #[default]
    Pending,
    Opening {
        anim: Animation,
        /// Unit vector pointing from the anchor towards where the popup ends up.
        direction: Point<f64, Logical>,
    },
    Open,
}

#[derive(Default)]
struct PopupOpenState(RefCell<State>);

fn with_state<T>(surface: &WlSurface, f: impl FnOnce(&mut State) -> T) -> T {
    with_states(surface, |states| {
        let state = states.data_map.get_or_insert(PopupOpenState::default);
        let mut state = state.0.borrow_mut();
        f(&mut state)
    })
}

/// Resets the state for a new popup role on this surface.
pub fn reset(surface: &WlSurface) {
    with_state(surface, |state| *state = State::Pending);
}

/// Starts the open animation on the popup's first commit with a buffer.
///
/// Returns `true` if an animation was started.
pub fn on_commit(popup: &PopupKind, clock: &Clock, config: niri_config::Animation) -> bool {
    // Input method popups follow the text cursor; leave them alone.
    let PopupKind::Xdg(xdg) = popup else {
        return false;
    };
    let surface = xdg.wl_surface();

    let has_buffer =
        with_renderer_surface_state(surface, |s| s.buffer().is_some()).unwrap_or(false);
    if !has_buffer {
        return false;
    }

    // Outside with_state(): this locks the same surface's states.
    let (geo, anchor) = xdg.with_pending_state(|s| (s.geometry, s.positioner.anchor_rect));
    // Placement may have flipped the popup, so look at where it ended up rather than at the
    // requested gravity. A popup covering its anchor only fades.
    let direction = if geo.loc.y >= anchor.loc.y + anchor.size.h {
        Point::from((0., 1.))
    } else if geo.loc.y + geo.size.h <= anchor.loc.y {
        Point::from((0., -1.))
    } else if geo.loc.x >= anchor.loc.x + anchor.size.w {
        Point::from((1., 0.))
    } else if geo.loc.x + geo.size.w <= anchor.loc.x {
        Point::from((-1., 0.))
    } else {
        Point::from((0., 0.))
    };

    with_state(surface, |state| {
        if !matches!(state, State::Pending) {
            return false;
        }

        if config.off {
            *state = State::Open;
            return false;
        }

        let anim = Animation::new(clock.clone(), 0., 1., 0., config);
        *state = State::Opening { anim, direction };
        true
    })
}

/// Returns the alpha multiplier and the offset to draw the popup at.
pub fn render_params(surface: &WlSurface) -> (f32, Point<f64, Logical>) {
    with_state(surface, |state| match state {
        State::Opening { anim, direction } => {
            let p = anim.clamped_value().clamp(0., 1.);
            let offset = direction.upscale(-TRAVEL * (1. - p));
            (p as f32, offset)
        }
        State::Pending | State::Open => (1., Point::from((0., 0.))),
    })
}

/// Advances the animation; returns whether it is still ongoing.
pub fn advance(surface: &WlSurface) -> bool {
    if !surface.is_alive() {
        return false;
    }

    with_state(surface, |state| match state {
        State::Opening { anim, .. } => {
            if anim.is_done() {
                *state = State::Open;
                false
            } else {
                true
            }
        }
        State::Pending | State::Open => false,
    })
}
