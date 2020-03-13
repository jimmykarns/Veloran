#![deny(unsafe_code)]
#![feature(drain_filter)]
#![recursion_limit = "2048"]

#[macro_use]
pub mod ui;
pub mod anim;
pub mod audio;
pub mod controller;
mod ecs;
pub mod error;
pub mod hud;
pub mod i18n;
pub mod key_state;
pub mod logging;
pub mod menu;
pub mod mesh;
pub mod meta;
pub mod render;
pub mod run;
pub mod scene;
pub mod session;
pub mod settings;
#[cfg(feature = "singleplayer")]
pub mod singleplayer;
pub mod window;

// Reexports
pub use crate::error::Error;

use crate::{
    audio::AudioFrontend,
    meta::Meta,
    render::Renderer,
    settings::Settings,
    singleplayer::Singleplayer,
    window::{Event, Window},
};
use common::{assets::watch, clock::Clock};

/// A type used to store state that is shared between all play states.
pub struct GlobalState {
    pub settings: Settings,
    pub meta: Meta,
    pub window: Window,
    pub audio: AudioFrontend,
    pub info_message: Option<String>,
    pub clock: Clock,
    #[cfg(feature = "singleplayer")]
    pub singleplayer: Option<Singleplayer>,
    // TODO: redo this so that the watcher doesn't have to exist for reloading to occur
    localization_watcher: watch::ReloadIndicator,
}

impl GlobalState {
    /// Called after a change in play state has occurred (usually used to
    /// reverse any temporary effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }

    pub fn maintain(&mut self, dt: f32) { self.audio.maintain(dt); }
}

pub enum Direction {
    Forwards,
    Backwards,
}

/// States can either close (and revert to a previous state), push a new state
/// on top of themselves, or switch to a totally different state.
pub enum PlayStateResult {
    /// Keep running this play state.
    Continue,
    /// Pop all play states in reverse order and shut down the program.
    Shutdown,
    /// Close the current play state and pop it from the play state stack.
    Pop,
    /// Push a new play state onto the play state stack.
    Push(Box<dyn PlayState>),
    /// Switch the current play state with a new play state.
    Switch(Box<dyn PlayState>),
}

/// A trait representing a playable game state. This may be a menu, a game
/// session, the title screen, etc.
pub trait PlayState {
    /// Get a descriptive name for this state type.
    /// Called when entering this play state from another
    fn enter(&mut self, global_state: &mut GlobalState, direction: Direction);

    /// Tick the play state
    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult;

    /// Get a descriptive name for this state type.
    fn name(&self) -> &'static str;

    /// Draw the play state.
    fn render(&mut self, renderer: &mut Renderer);
}
