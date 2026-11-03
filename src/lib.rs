//! Free Look -- a Horizon `.hmod` (WASM component, `horizon:mods@0.1.0`).
//!
//! Built against the very same SDK, WIT and manifest shape as the working
//! "Zoom (WI-Zoom)" `.hmod` pinned to Horizon `e7614d2`, so it loads on the
//! same runtime (and on the backwards-compatible Horizon Android runtimes).
//!
//! IMPORTANT: the sandboxed API has no way to set the camera's yaw/pitch or
//! perspective, and a mod's registered keys never consume the game's own
//! mapping. This mod therefore implements EVERYTHING the API allows:
//!
//! * English "Auto free look" and "Shift for free look" settings, saved with
//!   `storage` and restored on restart;
//! * F5, left Shift and right Shift registered with `input.register-key`
//!   (rebindable in Options > Mods);
//! * the exact F5 gating (both off = vanilla F5; Shift on = Shift+F5 only);
//! * the vertical angle limit and smooth return as a tested state machine;
//! * an English settings page, `/freelook`, and an honest HUD/chat status.
//!
//! It does NOT silently fake a rotating camera. `src/camera.rs` explains
//! exactly which host functions are missing; `docs/API-LIMITATIONS.md` lists
//! them with sources and `docs/NATIVE-API-PROPOSAL.md` specifies the native
//! API needed to finish the feature.

mod camera;

use free_look_logic::{F5Action, Phase, Rig, Settings, f5_action};
use horizon_mod_sdk::{
    Mod, Registration, chat, game, hud,
    input::{self, Event},
    storage,
    ui::{self, ButtonWidget, TextWidget, Widget},
    warn,
};

use crate::camera::{CameraOutput, SandboxedCamera, STATUS};

const KEY_FREE_LOOK: &str = "free_look";
const KEY_SHIFT_LEFT: &str = "shift_left";
const KEY_SHIFT_RIGHT: &str = "shift_right";

// Key names as options.txt writes them (Horizon's key_names.rs / keybinds.rs).
const DEFAULT_F5: &str = "key.keyboard.f5";
const DEFAULT_SHIFT_LEFT: &str = "key.keyboard.left.shift";
const DEFAULT_SHIFT_RIGHT: &str = "key.keyboard.right.shift";

const STORAGE_AUTO: &str = "auto_free_look";
const STORAGE_SHIFT: &str = "shift_for_free_look";

const STATUS_WIDGET: u32 = 1;
const AUTO_BUTTON: u32 = 2;
const SHIFT_BUTTON: u32 = 3;
const HELP_WIDGET: u32 = 4;

struct FreeLook {
    settings: Settings,
    rig: Rig,
    shift_held: bool,
    camera: SandboxedCamera,
    /// The one-time chat explanation has been shown.
    warned: bool,
    /// Avoids calling hud::clear() on every frame.
    panel_shown: bool,
}

impl FreeLook {
    /// English settings page in Options > Mods > Free Look > Settings.
    fn page(&self) {
        let on = |value: bool| if value { "ON" } else { "OFF" };
        ui::set_settings(&[
            Widget::Text(TextWidget {
                id: STATUS_WIDGET,
                text: "Free Look: settings and keys work; camera control needs a native Horizon API."
                    .into(),
                color: 0xFF5555,
            }),
            Widget::Button(ButtonWidget {
                id: AUTO_BUTTON,
                label: format!("Auto free look: {}", on(self.settings.auto_free_look)),
            }),
            Widget::Button(ButtonWidget {
                id: SHIFT_BUTTON,
                label: format!(
                    "Shift for free look: {}",
                    on(self.settings.shift_for_free_look)
                ),
            }),
            Widget::Text(TextWidget {
                id: HELP_WIDGET,
                text: "Both OFF: F5 is untouched. Shift ON: only Shift+F5 activates. F5 toggles."
                    .into(),
                color: 0,
            }),
        ]);
    }

    fn save(&self) {
        let flag = |value: bool| -> &'static [u8] {
            if value {
                b"true"
            } else {
                b"false"
            }
        };
        let _ = storage::set(STORAGE_AUTO, flag(self.settings.auto_free_look));
        let _ = storage::set(STORAGE_SHIFT, flag(self.settings.shift_for_free_look));
    }

    fn show_status(&mut self) {
        if !self.panel_shown {
            hud::set_panel(
                hud::Corner::TopRight,
                &[hud::Line {
                    text: STATUS.into(),
                    color: 0xFF5555,
                }],
            );
            self.panel_shown = true;
        }
    }

    fn hide_status(&mut self) {
        if self.panel_shown {
            hud::clear();
            self.panel_shown = false;
        }
    }

    fn explain_once(&mut self) {
        if !self.warned {
            self.warned = true;
            chat::show(STATUS);
        }
    }

    /// Called after a setting changes: with both settings off F5 must be
    /// completely untouched, including any pending free-look state.
    fn settings_changed(&mut self) {
        self.save();
        self.page();
        if !self.settings.intercepts_f5() && self.rig.is_active() {
            self.rig = Rig::new();
            let _ = self.camera.clear();
            self.hide_status();
        }
    }
}

fn stored_flag(key: &str) -> bool {
    // Anything other than exactly "false" counts as on; absent = false.
    storage::get(key)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| text.trim() != "false" && !text.trim().is_empty() && text.trim() != "0")
        .unwrap_or(false)
}

fn player_angles() -> (f32, f32) {
    match game::get_player() {
        Some(player) => (player.yaw, player.pitch),
        None => (0.0, 0.0),
    }
}

fn register_keys() -> Result<(), String> {
    input::register_key(KEY_FREE_LOOK, "Free Look (F5)", DEFAULT_F5)?;
    // Shift is optional for the F5 gate: if a build rejects one of these key
    // names, keep the mod loaded and log it instead of failing init().
    for (name, label, key) in [
        (KEY_SHIFT_LEFT, "Shift for free look (left)", DEFAULT_SHIFT_LEFT),
        (KEY_SHIFT_RIGHT, "Shift for free look (right)", DEFAULT_SHIFT_RIGHT),
    ] {
        if let Err(reason) = input::register_key(name, label, key) {
            warn(&format!("Free Look: could not register {key}: {reason}"));
        }
    }
    Ok(())
}

impl Mod for FreeLook {
    fn init() -> Result<(Self, Registration), String> {
        register_keys()?;
        warn(
            "Free Look: horizon:mods@0.1.0 has no camera-angle setter and no key consumption; \
             F5 keeps the game's perspective behaviour.",
        );

        let settings = Settings {
            auto_free_look: stored_flag(STORAGE_AUTO),
            shift_for_free_look: stored_flag(STORAGE_SHIFT),
        };
        let state = FreeLook {
            settings,
            rig: Rig::new(),
            shift_held: false,
            camera: SandboxedCamera,
            warned: false,
            panel_shown: false,
        };
        state.page();
        Ok((state, Registration::new().command("freelook").on_frame()))
    }

    fn frame(&mut self, seconds: f32) {
        let (player_yaw, player_pitch) = player_angles();

        for event in input::poll() {
            match event {
                Event::KeyDown(key)
                    if key == KEY_SHIFT_LEFT || key == KEY_SHIFT_RIGHT =>
                {
                    self.shift_held = true;
                }
                Event::KeyUp(key) if key == KEY_SHIFT_LEFT || key == KEY_SHIFT_RIGHT => {
                    self.shift_held = false;
                }
                Event::KeyDown(key) if key == KEY_FREE_LOOK => {
                    match f5_action(self.settings, self.shift_held, self.rig.is_active()) {
                        F5Action::Ignore => {
                            // Both settings off, or Shift is required and not
                            // held: vanilla F5 runs exactly as before.
                        }
                        F5Action::Activate => {
                            self.rig.activate(player_yaw, player_pitch);
                            // A real backend would take the view from here.
                            if !self.camera.set_view(self.rig.yaw, self.rig.pitch) {
                                self.explain_once();
                            }
                        }
                        F5Action::Deactivate => {
                            self.rig.deactivate();
                            let _ = self.camera.clear();
                        }
                    }
                }
                _ => {}
            }
        }

        // The smooth return and the vertical clamp run every frame. `step`
        // returns the angles a native camera backend would apply; on today's
        // sandboxed API they are simply not applied.
        if let Some((yaw, pitch)) = self.rig.step(seconds, player_yaw, player_pitch) {
            if !self.camera.set_view(yaw, pitch) {
                self.show_status();
            }
        } else {
            self.hide_status();
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "freelook" {
            return Err("Unknown command.".into());
        }
        let mut words = args.split_whitespace();
        let verb = words.next().unwrap_or("status");
        match verb {
            "status" => Ok(Some(format!(
                "Free Look: auto {}, shift {}, state {}. {}",
                if self.settings.auto_free_look { "ON" } else { "OFF" },
                if self.settings.shift_for_free_look {
                    "ON"
                } else {
                    "OFF"
                },
                match self.rig.phase {
                    Phase::Off => "off",
                    Phase::Active => "active",
                    Phase::Returning => "returning",
                },
                STATUS
            ))),
            "auto" | "shift" => {
                let value = words.next().unwrap_or("on");
                let on = !matches!(value, "off" | "false" | "0");
                if verb == "auto" {
                    self.settings.auto_free_look = on;
                } else {
                    self.settings.shift_for_free_look = on;
                }
                self.settings_changed();
                Ok(Some(format!(
                    "{} set to {}.",
                    if verb == "auto" {
                        "Auto free look"
                    } else {
                        "Shift for free look"
                    },
                    if on { "ON" } else { "OFF" }
                )))
            }
            "reset" => {
                self.settings = Settings::default();
                self.settings_changed();
                Ok(Some("Free Look settings reset (both OFF; F5 is vanilla's).".into()))
            }
            _ => Err("Usage: /freelook status | auto on|off | shift on|off | reset".into()),
        }
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            AUTO_BUTTON => self.settings.auto_free_look = !self.settings.auto_free_look,
            SHIFT_BUTTON => self.settings.shift_for_free_look = !self.settings.shift_for_free_look,
            _ => return,
        }
        self.settings_changed();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(FreeLook);
