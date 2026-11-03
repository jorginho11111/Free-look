//! Pure, host-independent free-look logic.
//!
//! This crate knows nothing about Horizon, WASM or the game. It holds every
//! rule that does not need an engine API:
//!
//! * which F5 presses the mod is allowed to react to (`Settings`, `f5_action`);
//! * the vertical angle limit that keeps the camera from flipping over;
//! * the shortest-path interpolation that would bring the camera smoothly
//!   back to the direction the player is actually facing.
//!
//! Applying the resulting angles needs an engine API that does not exist in
//! `horizon:mods@0.1.0`. See `src/camera.rs` and `docs/API-LIMITATIONS.md`.
//! The logic is kept complete and tested so that a native camera backend can
//! be plugged in without rewriting the mod.

#![forbid(unsafe_code)]

/// Maximum |pitch| in degrees. Horizon's pitch is -90 straight up, +90
/// straight down; 80 degrees keeps the horizon level and prevents a flip.
pub const MAX_PITCH: f32 = 80.0;

/// Time constant of the smooth return, in seconds. After ~5 tau the camera
/// is visually back on the player's direction.
pub const RETURN_TAU: f32 = 0.12;

/// Degrees of camera yaw/pitch per unit of pointer delta. A native backend
/// would scale this by the player's own sensitivity.
pub const DEFAULT_SENSITIVITY: f32 = 0.15;

/// Both settings off by default: installing the mod never changes F5.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// "Auto free look": F5 alone activates free look.
    pub auto_free_look: bool,
    /// "Shift for free look": only Shift + F5 activates free look.
    pub shift_for_free_look: bool,
}

impl Settings {
    /// Whether the mod reacts to F5 at all.
    pub fn intercepts_f5(self) -> bool {
        self.auto_free_look || self.shift_for_free_look
    }

    /// "Shift for free look" wins when both settings are on: it is the
    /// stricter rule, so a plain F5 keeps the game's normal behaviour.
    pub fn shift_required(self) -> bool {
        self.shift_for_free_look
    }

    /// Whether this F5 press may activate/deactivate free look.
    pub fn activation_allowed(self, shift_held: bool) -> bool {
        self.intercepts_f5() && (!self.shift_required() || shift_held)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F5Action {
    /// Leave F5 to the game (vanilla perspective switching).
    Ignore,
    /// Enter free look.
    Activate,
    /// Leave free look and start the smooth return.
    Deactivate,
}

/// The complete F5 decision. `active` includes the returning phase, so a
/// second F5 during the return cancels it instead of re-entering.
pub fn f5_action(settings: Settings, shift_held: bool, active: bool) -> F5Action {
    if !settings.activation_allowed(shift_held) {
        return F5Action::Ignore;
    }
    if active {
        F5Action::Deactivate
    } else {
        F5Action::Activate
    }
}

/// Wrap degrees into [0, 360).
pub fn wrap_degrees(degrees: f32) -> f32 {
    if !degrees.is_finite() {
        return 0.0;
    }
    degrees.rem_euclid(360.0)
}

/// The signed, shortest angular distance from `from` to `to`, in
/// (-180, 180]. Crossing the 180/-180 seam never spins the long way round.
pub fn shortest_delta(from: f32, to: f32) -> f32 {
    let mut delta = wrap_degrees(to) - wrap_degrees(from);
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta <= -180.0 {
        delta += 360.0;
    }
    delta
}

/// The vertical limit. Non-finite input becomes level.
pub fn clamp_pitch(pitch: f32) -> f32 {
    if !pitch.is_finite() {
        return 0.0;
    }
    pitch.clamp(-MAX_PITCH, MAX_PITCH)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The camera follows the player (vanilla behaviour).
    Off,
    /// Free look is on; the camera keeps its own angles.
    Active,
    /// Free look was switched off; angles ease back to the player's.
    Returning,
}

/// A camera that is independent of the player's own facing direction.
#[derive(Clone, Copy, Debug)]
pub struct Rig {
    pub phase: Phase,
    pub yaw: f32,
    pub pitch: f32,
    /// The player's direction, refreshed every frame while active. This is
    /// the target of the smooth return.
    pub home_yaw: f32,
    pub home_pitch: f32,
}

impl Default for Rig {
    fn default() -> Self {
        Self::new()
    }
}

impl Rig {
    pub fn new() -> Self {
        Self {
            phase: Phase::Off,
            yaw: 0.0,
            pitch: 0.0,
            home_yaw: 0.0,
            home_pitch: 0.0,
        }
    }

    pub fn is_active(self) -> bool {
        self.phase != Phase::Off
    }

    /// Start free look from the direction the player is currently facing.
    pub fn activate(&mut self, player_yaw: f32, player_pitch: f32) {
        self.phase = Phase::Active;
        self.yaw = wrap_degrees(player_yaw);
        self.pitch = clamp_pitch(player_pitch);
        self.home_yaw = self.yaw;
        self.home_pitch = self.pitch;
    }

    /// Begin the smooth return. Nothing snaps.
    pub fn deactivate(&mut self) {
        if self.phase == Phase::Active {
            self.phase = Phase::Returning;
        }
    }

    /// Rotate the free camera. `dx`/`dy` are pointer deltas; a native backend
    /// must supply them because `horizon:mods@0.1.0` has no mouse-motion event.
    pub fn look(&mut self, dx: f32, dy: f32, sensitivity: f32) {
        if self.phase != Phase::Active {
            return;
        }
        let sensitivity = if sensitivity.is_finite() {
            sensitivity.max(0.0)
        } else {
            DEFAULT_SENSITIVITY
        };
        self.yaw = wrap_degrees(self.yaw + dx * sensitivity);
        self.pitch = clamp_pitch(self.pitch + dy * sensitivity);
    }

    /// Advance one frame. Returns the angles a camera backend should apply,
    /// or `None` when the camera should be vanilla's again.
    pub fn step(&mut self, seconds: f32, player_yaw: f32, player_pitch: f32) -> Option<(f32, f32)> {
        let target_yaw = wrap_degrees(player_yaw);
        let target_pitch = clamp_pitch(player_pitch);
        match self.phase {
            Phase::Off => {
                self.yaw = target_yaw;
                self.pitch = target_pitch;
                self.home_yaw = target_yaw;
                self.home_pitch = target_pitch;
                None
            }
            Phase::Active => {
                // The player keeps turning and walking in their own direction;
                // remember it as the return target, but never follow it.
                self.home_yaw = target_yaw;
                self.home_pitch = target_pitch;
                Some((self.yaw, self.pitch))
            }
            Phase::Returning => {
                let dt = if seconds.is_finite() { seconds.max(0.0) } else { 0.0 };
                let alpha = 1.0 - (-dt / RETURN_TAU).exp();
                self.yaw = wrap_degrees(self.yaw + shortest_delta(self.yaw, target_yaw) * alpha);
                self.pitch += (target_pitch - self.pitch) * alpha;
                let yaw_error = shortest_delta(self.yaw, target_yaw).abs();
                let pitch_error = (self.pitch - target_pitch).abs();
                if yaw_error < 0.05 && pitch_error < 0.05 {
                    self.phase = Phase::Off;
                    self.yaw = target_yaw;
                    self.pitch = target_pitch;
                    None
                } else {
                    Some((self.yaw, self.pitch))
                }
            }
        }
    }

    /// The angles that would be pushed to the camera right now.
    pub fn view(self) -> Option<(f32, f32)> {
        match self.phase {
            Phase::Off => None,
            Phase::Active | Phase::Returning => Some((self.yaw, self.pitch)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTO: Settings = Settings {
        auto_free_look: true,
        shift_for_free_look: false,
    };
    const SHIFT: Settings = Settings {
        auto_free_look: false,
        shift_for_free_look: true,
    };
    const BOTH: Settings = Settings {
        auto_free_look: true,
        shift_for_free_look: true,
    };
    const OFF: Settings = Settings {
        auto_free_look: false,
        shift_for_free_look: false,
    };

    #[test]
    fn both_off_leaves_f5_to_the_game() {
        assert!(!OFF.intercepts_f5());
        assert_eq!(f5_action(OFF, false, false), F5Action::Ignore);
        assert_eq!(f5_action(OFF, true, true), F5Action::Ignore);
    }

    #[test]
    fn auto_free_look_uses_plain_f5() {
        assert_eq!(f5_action(AUTO, false, false), F5Action::Activate);
        assert_eq!(f5_action(AUTO, false, true), F5Action::Deactivate);
        assert_eq!(f5_action(AUTO, true, false), F5Action::Activate);
    }

    #[test]
    fn shift_for_free_look_needs_shift_and_plain_f5_is_vanilla() {
        assert_eq!(f5_action(SHIFT, false, false), F5Action::Ignore);
        assert_eq!(f5_action(SHIFT, true, false), F5Action::Activate);
        assert_eq!(f5_action(SHIFT, true, true), F5Action::Deactivate);
    }

    #[test]
    fn shift_wins_when_both_settings_are_on() {
        assert_eq!(f5_action(BOTH, false, false), F5Action::Ignore);
        assert_eq!(f5_action(BOTH, true, false), F5Action::Activate);
    }

    #[test]
    fn pitch_is_clamped_so_the_camera_never_flips() {
        assert_eq!(clamp_pitch(120.0), MAX_PITCH);
        assert_eq!(clamp_pitch(-120.0), -MAX_PITCH);
        assert_eq!(clamp_pitch(35.0), 35.0);
        assert_eq!(clamp_pitch(f32::NAN), 0.0);
        let mut rig = Rig::new();
        rig.activate(0.0, 0.0);
        rig.look(0.0, 10_000.0, DEFAULT_SENSITIVITY);
        assert_eq!(rig.pitch, MAX_PITCH);
        rig.look(0.0, -20_000.0, DEFAULT_SENSITIVITY);
        assert_eq!(rig.pitch, -MAX_PITCH);
    }

    #[test]
    fn yaw_uses_the_shortest_path_across_the_seam() {
        assert_eq!(shortest_delta(350.0, 10.0), 20.0);
        assert_eq!(shortest_delta(10.0, 350.0), -20.0);
        assert_eq!(shortest_delta(-170.0, 170.0), -20.0);
        assert_eq!(wrap_degrees(-90.0), 270.0);
        assert_eq!(wrap_degrees(370.0), 10.0);
    }

    #[test]
    fn smooth_return_converges_without_a_snap() {
        let mut rig = Rig::new();
        rig.activate(0.0, 0.0);
        rig.look(600.0, 0.0, DEFAULT_SENSITIVITY); // 90 degrees away
        assert_eq!(rig.yaw, 90.0);
        assert_eq!(rig.phase, Phase::Active);
        rig.deactivate();
        assert_eq!(rig.phase, Phase::Returning);
        // One frame must not jump straight to the target.
        let first = rig.step(1.0 / 60.0, 0.0, 0.0).unwrap();
        assert!(first.0 > 1.0 && first.0 < 90.0);
        for _ in 0..120 {
            rig.step(1.0 / 60.0, 0.0, 0.0);
        }
        assert_eq!(rig.phase, Phase::Off);
        assert_eq!(rig.view(), None);
    }

    #[test]
    fn return_target_is_the_players_live_direction() {
        let mut rig = Rig::new();
        rig.activate(0.0, 0.0);
        rig.look(-600.0, 200.0, DEFAULT_SENSITIVITY); // yaw 270, pitch 30
        rig.deactivate();
        for _ in 0..120 {
            rig.step(1.0 / 60.0, 100.0, -20.0);
        }
        assert_eq!(rig.phase, Phase::Off);
        assert!((rig.yaw - 100.0).abs() < 0.1, "yaw {}", rig.yaw);
        assert!((rig.pitch - (-20.0)).abs() < 0.1, "pitch {}", rig.pitch);
    }

    #[test]
    fn player_turning_does_not_move_the_free_camera() {
        let mut rig = Rig::new();
        rig.activate(10.0, 5.0);
        for _ in 0..30 {
            rig.step(1.0 / 60.0, 180.0, -40.0);
        }
        let (yaw, pitch) = rig.view().unwrap();
        assert_eq!(yaw, 10.0);
        assert_eq!(pitch, 5.0);
        assert_eq!(rig.home_yaw, 180.0);
        assert_eq!(rig.home_pitch, -40.0);
    }
}
