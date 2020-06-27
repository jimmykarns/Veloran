#![deny(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]
#![feature(bool_to_option)]
#![recursion_limit = "2048"]

use veloren_voxygen::{
    audio::{self, AudioFrontend},
    i18n::{self, i18n_asset_key, Localization},
    logging,
    menu::main::MainMenuState,
    profile::Profile,
    settings::{AudioOutput, Settings},
    window::Window,
    Direction, GlobalState, PlayState, PlayStateResult,
};

use common::assets::{load, load_expect};
use std::{mem, panic};
use tracing::{debug, error, warn};

fn main() {
    #[cfg(feature = "tweak")]
    const_tweaker::run().expect("Could not run server");

    // Load the settings
    // Note: This won't log anything due to it being called before
    // `logging::init`. The issue is we need to read a setting to decide
    // whether we create a log file or not.
    let settings = Settings::load();

    // Init logging and hold the guards.
    let _guards = logging::init(&settings);

    // Save settings to add new fields or create the file if it is not already
    // there.
    if let Err(err) = settings.save_to_file() {
        panic!("Failed to save settings: {:?}", err);
    }

    let mut audio = match settings.audio.output {
        AudioOutput::Off => None,
        AudioOutput::Automatic => audio::get_default_device(),
        AudioOutput::Device(ref dev) => Some(dev.clone()),
    }
    .map(|dev| AudioFrontend::new(dev, settings.audio.max_sfx_channels))
    .unwrap_or_else(AudioFrontend::no_audio);

    audio.set_music_volume(settings.audio.music_volume);
    audio.set_sfx_volume(settings.audio.sfx_volume);

    // Load the profile.
    let profile = Profile::load();

    let mut global_state = GlobalState {
        audio,
        profile,
        window: Window::new(&settings).expect("Failed to create window!"),
        settings,
        info_message: None,
        singleplayer: None,
    };

    // Try to load the localization and log missing entries
    let localized_strings = load::<Localization>(&i18n_asset_key(
        &global_state.settings.language.selected_language,
    ))
    .unwrap_or_else(|e| {
        let preferred_language = &global_state.settings.language.selected_language;
        warn!(
            ?e,
            ?preferred_language,
            "Impossible to load language: change to the default language (English) instead.",
        );
        global_state.settings.language.selected_language = i18n::REFERENCE_LANG.to_owned();
        load_expect::<Localization>(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ))
    });
    localized_strings.log_missing_entries();

    // Set up panic handler to relay swish panic messages to the user
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let panic_info_payload = panic_info.payload();
        let payload_string = panic_info_payload.downcast_ref::<String>();
        let reason = match payload_string {
            Some(s) => &s,
            None => {
                let payload_str = panic_info_payload.downcast_ref::<&str>();
                match payload_str {
                    Some(st) => st,
                    None => "Payload is not a string",
                }
            },
        };
        let msg = format!(
            "A critical error has occurred and Voxygen has been forced to \
            terminate in an unusual manner. Details about the error can be \
            found below.\n\
            \n\
            > What should I do?\n\
            \n\
            We need your help to fix this! You can help by contacting us and \
            reporting this problem. To do this, open an issue on the Veloren \
            issue tracker:\n\
            \n\
            https://www.gitlab.com/veloren/veloren/issues/new\n\
            \n\
            If you're on the Veloren community Discord server, we'd be \
            grateful if you could also post a message in the #support channel.
            \n\
            > What should I include?\n\
            \n\
            The error information below will be useful in finding and fixing \
            the problem. Please include as much information about your setup \
            and the events that led up to the panic as possible.
            \n\
            Voxygen has logged information about the problem (including this \
            message) to the file {}. Please include the contents of this \
            file in your bug report.
            \n\
            > Error information\n\
            \n\
            The information below is intended for developers and testers.\n\
            \n\
            Panic Payload: {:?}\n\
            PanicInfo: {}\n\
            Game version: {} [{}]",
            Settings::load()
                .log
                .logs_path
                .join("voxygen-<date>.log")
                .display(),
            reason,
            panic_info,
            common::util::GIT_HASH.to_string(),
            common::util::GIT_DATE.to_string()
        );

        error!(
            "VOXYGEN HAS PANICKED\n\n{}\n\nBacktrace:\n{:?}",
            msg,
            backtrace::Backtrace::new(),
        );

        #[cfg(feature = "msgbox")]
        {
            #[cfg(target_os = "macos")]
            dispatch::Queue::main()
                .sync(|| msgbox::create("Voxygen has panicked", &msg, msgbox::IconType::Error));
            #[cfg(not(target_os = "macos"))]
            msgbox::create("Voxygen has panicked", &msg, msgbox::IconType::Error);
        }

        default_hook(panic_info);
    }));

    // Initialise watcher for animation hotreloading
    #[cfg(feature = "hot-anim")]
    anim::init();

    // Set up the initial play state.
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(MainMenuState::new(&mut global_state))];
    states.last().map(|current_state| {
        let current_state = current_state.name();
        debug!(?current_state, "Started game with state")
    });

    // What's going on here?
    // ---------------------
    // The state system used by Voxygen allows for the easy development of
    // stack-based menus. For example, you may want a "title" state that can
    // push a "main menu" state on top of it, which can in turn push a
    // "settings" state or a "game session" state on top of it. The code below
    // manages the state transfer logic automatically so that we don't have to
    // re-engineer it for each menu we decide to add to the game.
    let mut direction = Direction::Forwards;
    while let Some(state_result) = states
        .last_mut()
        .map(|last| last.play(direction, &mut global_state))
    {
        // Implement state transfer logic.
        match state_result {
            PlayStateResult::Shutdown => {
                direction = Direction::Backwards;
                debug!("Shutting down all states...");
                while states.last().is_some() {
                    states.pop().map(|old_state| {
                        let old_state = old_state.name();
                        debug!(?old_state, "Popped state");
                        global_state.on_play_state_changed();
                    });
                }
            },
            PlayStateResult::Pop => {
                direction = Direction::Backwards;
                states.pop().map(|old_state| {
                    let old_state = old_state.name();
                    debug!(?old_state, "Popped state");
                    global_state.on_play_state_changed();
                });
            },
            PlayStateResult::Push(new_state) => {
                direction = Direction::Forwards;
                debug!("Pushed state '{}'.", new_state.name());
                states.push(new_state);
                global_state.on_play_state_changed();
            },
            PlayStateResult::Switch(mut new_state_box) => {
                direction = Direction::Forwards;
                states.last_mut().map(|old_state_box| {
                    let old_state = old_state_box.name();
                    let new_state = new_state_box.name();
                    debug!(?old_state, ?new_state, "Switching to states",);
                    mem::swap(old_state_box, &mut new_state_box);
                    global_state.on_play_state_changed();
                });
            },
        }
    }

    // Save any unsaved changes to profile.
    global_state.profile.save_to_file_warn();
    // Save any unsaved changes to settings.
    global_state.settings.save_to_file_warn();
}
