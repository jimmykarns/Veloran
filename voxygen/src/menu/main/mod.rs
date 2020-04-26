mod client_init;
mod ui;

use super::char_selection::CharSelectionState;
#[cfg(feature = "singleplayer")]
use crate::singleplayer::Singleplayer;
use crate::{
    window::Event, Direction, FirstDrawer, GlobalState, PlayState, PlayStateResult, SecondDrawer,
};
use client_init::{ClientInit, Error as InitError, Msg as InitMsg};
use common::{assets::load_expect, comp};
use log::{error, warn};
use ui::{Event as MainMenuEvent, MainMenuUi};

pub struct MainMenuState {
    main_menu_ui: MainMenuUi,
    // Used for client creation.
    client_init: Option<ClientInit>,
}

impl MainMenuState {
    /// Create a new `MainMenuState`.
    pub fn new(global_state: &mut GlobalState) -> Self {
        Self {
            main_menu_ui: MainMenuUi::new(global_state),
            client_init: None,
        }
    }
}

const DEFAULT_PORT: u16 = 14004;

impl PlayState for MainMenuState {
    fn enter(&mut self, global_state: &mut GlobalState, _: Direction) {
        // Kick off title music
        if global_state.settings.audio.audio_on && global_state.audio.music_enabled() {
            global_state.audio.play_title_music();
        }

        // Reset singleplayer server if it was running already
        #[cfg(feature = "singleplayer")]
        {
            global_state.singleplayer = None;
        }
    }

    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult {
        let localized_strings = load_expect::<crate::i18n::VoxygenLocalization>(
            &crate::i18n::i18n_asset_key(&global_state.settings.language.selected_language),
        );

        // Handle window events.
        for event in events {
            match event {
                Event::Close => return PlayStateResult::Shutdown,
                // Pass events to ui.
                Event::Ui(event) => {
                    self.main_menu_ui.handle_event(event);
                },
                // Ignore all other events.
                _ => {},
            }
        }
        // Poll client creation.
        match self.client_init.as_ref().and_then(|init| init.poll()) {
            Some(InitMsg::Done(Ok(mut client))) => {
                self.main_menu_ui.connected();
                // Register voxygen components / resources
                crate::ecs::init(client.state_mut().ecs_mut());
                return PlayStateResult::Push(Box::new(CharSelectionState::new(
                    global_state,
                    std::rc::Rc::new(std::cell::RefCell::new(client)),
                )));
            },
            Some(InitMsg::Done(Err(err))) => {
                self.client_init = None;
                global_state.info_message = Some({
                    let err = match err {
                        InitError::BadAddress(_) | InitError::NoAddress => {
                            localized_strings.get("main.login.server_not_found").into()
                        },
                        InitError::ClientError(err) => match err {
                            client::Error::AuthErr(e) => format!(
                                "{}: {}",
                                localized_strings.get("main.login.authentication_error"),
                                e
                            ),
                            client::Error::TooManyPlayers => {
                                localized_strings.get("main.login.server_full").into()
                            },
                            client::Error::AuthServerNotTrusted => localized_strings
                                .get("main.login.untrusted_auth_server")
                                .into(),
                            client::Error::ServerWentMad => localized_strings
                                .get("main.login.outdated_client_or_server")
                                .into(),
                            client::Error::ServerTimeout => {
                                localized_strings.get("main.login.timeout").into()
                            },
                            client::Error::ServerShutdown => {
                                localized_strings.get("main.login.server_shut_down").into()
                            },
                            client::Error::AlreadyLoggedIn => {
                                localized_strings.get("main.login.already_logged_in").into()
                            },
                            client::Error::Network(e) => format!(
                                "{}: {:?}",
                                localized_strings.get("main.login.network_error"),
                                e
                            ),
                            client::Error::Other(e) => {
                                format!("{}: {}", localized_strings.get("common.error"), e)
                            },
                            client::Error::AuthClientError(e) => match e {
                                client::AuthClientError::JsonError(e) => format!(
                                    "{}: {}",
                                    localized_strings.get("common.fatal_error"),
                                    e
                                ),
                                client::AuthClientError::RequestError(_) => format!(
                                    "{}: {}",
                                    localized_strings.get("main.login.failed_sending_request"),
                                    e
                                ),
                                client::AuthClientError::ServerError(_, e) => format!("{}", e),
                            },
                        },
                        InitError::ClientCrashed => {
                            localized_strings.get("main.login.client_crashed").into()
                        },
                    };
                    // Log error for possible additional use later or incase that the error
                    // displayed is cut of.
                    error!("{}", err);
                    err
                });
            },
            Some(InitMsg::IsAuthTrusted(auth_server)) => {
                if global_state
                    .settings
                    .networking
                    .trusted_auth_servers
                    .contains(&auth_server)
                {
                    // Can't fail since we just polled it, it must be Some
                    self.client_init
                        .as_ref()
                        .unwrap()
                        .auth_trust(auth_server, true);
                } else {
                    // Show warning that auth server is not trusted and prompt for approval
                    self.main_menu_ui.auth_trust_prompt(auth_server);
                }
            },
            None => {},
        }

        // Maintain the UI.
        for event in self
            .main_menu_ui
            .maintain(global_state, global_state.clock.get_last_delta())
        {
            match event {
                MainMenuEvent::LoginAttempt {
                    username,
                    password,
                    server_address,
                } => {
                    attempt_login(
                        global_state,
                        username,
                        password,
                        server_address,
                        DEFAULT_PORT,
                        &mut self.client_init,
                    );
                },
                MainMenuEvent::CancelLoginAttempt => {
                    // client_init contains Some(ClientInit), which spawns a thread which contains a
                    // TcpStream::connect() call This call is blocking
                    // TODO fix when the network rework happens
                    #[cfg(feature = "singleplayer")]
                    {
                        global_state.singleplayer = None;
                    }
                    self.client_init = None;
                    self.main_menu_ui.cancel_connection();
                },
                #[cfg(feature = "singleplayer")]
                MainMenuEvent::StartSingleplayer => {
                    let (singleplayer, server_settings) = Singleplayer::new(None); // TODO: Make client and server use the same thread pool

                    global_state.singleplayer = Some(singleplayer);

                    attempt_login(
                        global_state,
                        "singleplayer".to_owned(),
                        "".to_owned(),
                        server_settings.gameserver_address.ip().to_string(),
                        server_settings.gameserver_address.port(),
                        &mut self.client_init,
                    );
                },
                MainMenuEvent::Settings => {}, // TODO
                MainMenuEvent::Quit => return PlayStateResult::Shutdown,
                MainMenuEvent::DisclaimerClosed => global_state.settings.show_disclaimer = false,
                MainMenuEvent::AuthServerTrust(auth_server, trust) => {
                    if trust {
                        global_state
                            .settings
                            .networking
                            .trusted_auth_servers
                            .insert(auth_server.clone());
                        global_state.settings.save_to_file_warn();
                    }
                    self.client_init
                        .as_ref()
                        .map(|init| init.auth_trust(auth_server, trust));
                },
            }
        }

        if let Some(info) = global_state.info_message.take() {
            self.main_menu_ui.show_info(info);
        }

        PlayStateResult::Continue
    }

    fn name(&self) -> &'static str { "Title" }

    fn first_render(&mut self, _: &mut FirstDrawer) {}

    fn second_render<'b>(&'b mut self, drawer: &'b mut SecondDrawer<'b>) {
        // Draw the UI to the screen.
        self.main_menu_ui.render(drawer);
    }
}

fn attempt_login(
    global_state: &mut GlobalState,
    username: String,
    password: String,
    server_address: String,
    server_port: u16,
    client_init: &mut Option<ClientInit>,
) {
    let mut net_settings = &mut global_state.settings.networking;
    net_settings.username = username.clone();
    net_settings.password = password.clone();
    if !net_settings.servers.contains(&server_address) {
        net_settings.servers.push(server_address.clone());
    }
    if let Err(err) = global_state.settings.save_to_file() {
        warn!("Failed to save settings: {:?}", err);
    }

    if comp::Player::alias_is_valid(&username) {
        // Don't try to connect if there is already a connection in progress.
        if client_init.is_none() {
            *client_init = Some(ClientInit::new(
                (server_address, server_port, false),
                username,
                Some(global_state.settings.graphics.view_distance),
                { password },
            ));
        }
    } else {
        global_state.info_message = Some("Invalid username".to_string());
    }
}
