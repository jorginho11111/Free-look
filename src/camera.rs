//! The camera output of Free Look -- and why it is intentionally a no-op.
//!
//! `horizon:mods@0.1.0` (Horizon `e7614d2`, and the same WIT in the Horizon
//! Android 0.7.8 dev builds) offers exactly these camera-adjacent functions:
//!
//! * `render.set-zoom(zoom)` -- divides the field of view by a factor;
//! * `render.shake(strength, seconds)` -- a decaying, undirected jolt;
//! * `render.set-minimap`, `render.set-far-terrain`, `render.set-hud` -- 2D
//!   overlays, not the view;
//! * `game.get-player()` -- a READ-ONLY snapshot with yaw and pitch.
//!
//! None of them sets where the camera points, none changes the perspective
//! mode, and `input.register-key`'s events never consume the game's own
//! mapping ("a mod's keys never consume" in the official docs). There is also
//! no pointer-motion event to steer a detached camera with.
//!
//! So this module does NOT invent a `set-camera` call. `SandboxedCamera`
//! reports `false` for every attempt, which the main module turns into an
//! honest English status line instead of pretending free look is running.
//! When a native `freelook` API exists (see docs/NATIVE-API-PROPOSAL.md), the
//! only change needed here is a second implementation of `CameraOutput`.

/// English, in-game text explaining the missing engine feature.
pub const STATUS: &str =
    "Free Look: camera control is unavailable in this Horizon build. F5 keeps the game's perspective.";

/// What a real camera backend would have to do.
pub trait CameraOutput {
    /// Point the camera at `(yaw, pitch)` without changing the player's yaw,
    /// pitch, movement direction or the F5 perspective cycle.
    /// Returns `false` when the host cannot do it.
    fn set_view(&self, yaw: f32, pitch: f32) -> bool;

    /// Hand the camera back to the game (after the smooth return).
    /// Returns `false` when the host cannot do it.
    fn clear(&self) -> bool;
}

/// The backend for the sandboxed `.hmod` API as it exists today.
#[derive(Clone, Copy, Debug, Default)]
pub struct SandboxedCamera;

impl CameraOutput for SandboxedCamera {
    fn set_view(&self, _yaw: f32, _pitch: f32) -> bool {
        // No host call exists. Calling `render.set-zoom` here would only
        // narrow the FOV and would be a lie; calling `render.shake` would be
        // an uncontrollable jolt. Neither is free look.
        false
    }

    fn clear(&self) -> bool {
        false
    }
}
