mod connecting;
//mod disclaimer;
mod login;
mod servers;

use crate::{
    i18n::{i18n_asset_key, Localization},
    render::Renderer,
    ui::{
        self,
        fonts::{Fonts, IcedFonts},
        ice::{Element, IcedUi},
        img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic},
        Graphic, Ui,
    },
    GlobalState,
};
//ImageFrame, Tooltip,
use crate::settings::Settings;
use common::assets::load_expect;
use conrod_core::{
    color,
    color::TRANSPARENT,
    position::Relative,
    widget::{text_box::Event as TextBoxEvent, Button, Image, List, Rectangle, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use rand::{seq::SliceRandom, thread_rng};
use std::time::Duration;
use ui::ice::widget;

const COL1: Color = Color::Rgba(0.07, 0.1, 0.1, 0.9);

// UI Color-Theme
/*const UI_MAIN: Color = Color::Rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue
const UI_HIGHLIGHT_0: Color = Color::Rgba(0.79, 1.09, 1.09, 1.0);*/

use iced::{text_input, Column, Container, HorizontalAlignment, Length};
use ui::ice::style;
image_ids_ice! {
    struct IcedImgs {
        <VoxelGraphic>
        v_logo: "voxygen.element.v_logo",

        info_frame: "voxygen.element.frames.info_frame_2",

        //banner: "voxygen.element.frames.banner",
        <ImageGraphic>
        bg: "voxygen.background.bg_main",
        banner: "voxygen.element.frames.banner_png",
        banner_bottom: "voxygen.element.frames.banner_bottom_png",
        banner_top: "voxygen.element.frames.banner_top",
        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",
        input_bg: "voxygen.element.misc_bg.textbox",
        disclaimer: "voxygen.element.frames.disclaimer",

        <BlankGraphic>
        nothing: (),
    }
}

// Randomly loaded background images
const BG_IMGS: [&str; 16] = [
    "voxygen.background.bg_1",
    "voxygen.background.bg_2",
    "voxygen.background.bg_3",
    "voxygen.background.bg_4",
    "voxygen.background.bg_5",
    "voxygen.background.bg_6",
    "voxygen.background.bg_7",
    "voxygen.background.bg_8",
    "voxygen.background.bg_9",
    "voxygen.background.bg_10",
    "voxygen.background.bg_11",
    "voxygen.background.bg_12",
    "voxygen.background.bg_13",
    "voxygen.background.bg_14",
    "voxygen.background.bg_15",
    "voxygen.background.bg_16",
];

pub enum Event {
    LoginAttempt {
        username: String,
        password: String,
        server_address: String,
    },
    CancelLoginAttempt,
    #[cfg(feature = "singleplayer")]
    StartSingleplayer,
    Quit,
    Settings,
    //DisclaimerClosed, TODO: remove all traces?
    //DisclaimerAccepted,
    AuthServerTrust(String, bool),
}

pub enum PopupType {
    Error,
    ConnectionInfo,
    AuthTrustPrompt(String),
}

pub struct PopupData {
    msg: String,
    popup_type: PopupType,
}

pub struct LoginInfo {
    pub username: String,
    pub password: String,
    pub server: String,
}

enum ConnectionState {
    InProgress {
        status: String,
    },
    AuthTrustPrompt {
        auth_server: String,
        msg: String,
        // To remember when we switch back
        status: String,
    },
}
impl ConnectionState {
    fn take_status_string(&mut self) -> String {
        std::mem::take(match self {
            Self::InProgress { status } => status,
            Self::AuthTrustPrompt { status, .. } => status,
        })
    }
}

enum Screen {
    /*Disclaimer {
        screen: disclaimer::Screen,
    },*/
    Login {
        screen: login::Screen,
        // Error to display in a box
        error: Option<String>,
    },
    Servers {
        screen: servers::Screen,
    },
    Connecting {
        screen: connecting::Screen,
        connection_state: ConnectionState,
    },
}

struct IcedState {
    fonts: IcedFonts,
    imgs: IcedImgs,
    bg_img: widget::image::Handle,
    i18n: std::sync::Arc<Localization>,
    // Voxygen version
    version: String,

    servers: Vec<String>,
    selected_server_index: Option<usize>,
    login_info: LoginInfo,

    time: f32,

    screen: Screen,
}

#[derive(Clone)]
enum Message {
    Quit,
    Back,
    ShowServers,
    #[cfg(feature = "singleplayer")]
    Singleplayer,
    Multiplayer,
    Username(String),
    Password(String),
    Server(String),
    ServerChanged(usize),
    FocusPassword,
    CancelConnect,
    TrustPromptAdd,
    TrustPromptCancel,
    CloseError,
    /*CloseDisclaimer,
     *AcceptDisclaimer, */
}

impl IcedState {
    fn new(
        fonts: IcedFonts,
        imgs: IcedImgs,
        bg_img: widget::image::Handle,
        i18n: std::sync::Arc<Localization>,
        settings: &Settings,
    ) -> Self {
        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );

        let screen = /* if settings.show_disclaimer {
            Screen::Disclaimer {
                screen: disclaimer::Screen::new(),
            }
        } else { */
            Screen::Login {
                screen: login::Screen::new(),
                error: None,
            };
        //};

        let servers = settings.networking.servers.clone();
        let login_info = LoginInfo {
            username: settings.networking.username.clone(),
            password: String::new(),
            server: settings.networking.default_server.clone(),
        };
        let selected_server_index = servers.iter().position(|f| f == &login_info.server);

        Self {
            fonts,
            imgs,
            bg_img,
            i18n,
            version,

            servers,
            selected_server_index,
            login_info,

            time: 0.0,

            screen,
        }
    }

    fn view(&mut self, dt: f32) -> Element<Message> {
        self.time = self.time + dt;

        // TODO: consider setting this as the default in the renderer
        let button_style = style::button::Style::new(self.imgs.button)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .text_color(login::TEXT_COLOR)
            .disabled_text_color(login::DISABLED_TEXT_COLOR);

        let version = iced::Text::new(&self.version)
            .size(self.fonts.cyri.scale(15))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Right);

        let bg_img = if matches!(&self.screen, Screen::Connecting {..}) {
            self.bg_img
        } else {
            self.imgs.bg
        };

        // TODO: make any large text blocks scrollable so that if the area is to
        // small they can still be read
        let content = match &mut self.screen {
            /*Screen::Disclaimer { screen } => {
                screen.view(&self.fonts, &self.imgs, &self.i18n, button_style)
            },*/
            Screen::Login { screen, error } => screen.view(
                &self.fonts,
                &self.imgs,
                &self.login_info,
                error.as_deref(),
                &self.i18n,
                button_style,
            ),
            Screen::Servers { screen } => screen.view(
                &self.fonts,
                &self.imgs,
                &self.servers,
                self.selected_server_index,
                &self.i18n,
                button_style,
            ),
            Screen::Connecting {
                screen,
                connection_state,
            } => screen.view(
                &self.fonts,
                &self.imgs,
                &connection_state,
                self.time,
                &self.i18n,
                button_style,
            ),
        };

        Container::new(
            Column::with_children(vec![version.into(), content])
                .spacing(3)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(style::container::Style::image(bg_img))
        .padding(3)
        .into()
    }

    fn update(&mut self, message: Message, events: &mut Vec<Event>) {
        match message {
            Message::Quit => events.push(Event::Quit),
            Message::Back => {
                self.screen = Screen::Login {
                    screen: login::Screen::new(),
                    error: None,
                };
            },
            Message::ShowServers => {
                self.screen = Screen::Servers {
                    screen: servers::Screen::new(),
                };
            },
            #[cfg(feature = "singleplayer")]
            Message::Singleplayer => {
                self.screen = Screen::Connecting {
                    screen: connecting::Screen::new(),
                    connection_state: ConnectionState::InProgress {
                        status: [self.i18n.get("main.creating_world"), "..."].concat(),
                    },
                };

                events.push(Event::StartSingleplayer);
            },
            Message::Multiplayer => {
                self.screen = Screen::Connecting {
                    screen: connecting::Screen::new(),
                    connection_state: ConnectionState::InProgress {
                        status: [self.i18n.get("main.connecting"), "..."].concat(),
                    },
                };

                events.push(Event::LoginAttempt {
                    username: self.login_info.username.clone(),
                    password: self.login_info.password.clone(),
                    server_address: self.login_info.server.clone(),
                });
            },
            Message::Username(new_value) => self.login_info.username = new_value,
            Message::Password(new_value) => self.login_info.password = new_value,
            Message::Server(new_value) => {
                self.selected_server_index = self.servers.iter().position(|f| f == &new_value);
                self.login_info.server = new_value;
            },
            Message::ServerChanged(new_value) => {
                self.login_info.server = self.servers[new_value].clone();
                self.selected_server_index = Some(new_value);
            },
            Message::FocusPassword => {
                if let Screen::Login { screen, .. } = &mut self.screen {
                    screen.banner.password = text_input::State::focused();
                    screen.banner.username = text_input::State::new();
                }
            },
            Message::CancelConnect => {
                self.exit_connect_screen();
                events.push(Event::CancelLoginAttempt);
            },
            msg @ Message::TrustPromptAdd | msg @ Message::TrustPromptCancel => {
                if let Screen::Connecting {
                    connection_state, ..
                } = &mut self.screen
                {
                    if let ConnectionState::AuthTrustPrompt {
                        auth_server,
                        status,
                        ..
                    } = connection_state
                    {
                        let auth_server = std::mem::take(auth_server);
                        let status = std::mem::take(status);
                        let added = matches!(msg, Message::TrustPromptAdd);

                        *connection_state = ConnectionState::InProgress { status };
                        events.push(Event::AuthServerTrust(auth_server, added));
                    }
                }
            },
            Message::CloseError => {
                if let Screen::Login { error, .. } = &mut self.screen {
                    *error = None;
                }
            },
            //Message::CloseDisclaimer => {
            //   events.push(Event::DisclaimerClosed);
            //},
            /*Message::AcceptDisclaimer => {
                if let Screen::Disclaimer { .. } = &self.screen {
                    events.push(Event::DisclaimerAccepted);
                    self.screen = Screen::Login {
                        screen: login::Screen::new(),
                        error: None,
                    };
                }
            },*/
        }
    }

    // Connection successful of failed
    fn exit_connect_screen(&mut self) {
        if matches!(&self.screen, Screen::Connecting {..}) {
            self.screen = Screen::Login {
                screen: login::Screen::new(),
                error: None,
            }
        }
    }

    fn auth_trust_prompt(&mut self, auth_server: String) {
        if let Screen::Connecting {
            connection_state, ..
        } = &mut self.screen
        {
            let msg = format!(
                "Warning: The server you are trying to connect to has provided this \
                 authentication server address:\n\n{}\n\nbut it is not in your list of trusted \
                 authentication servers.\n\nMake sure that you trust this site and owner to not \
                 try and bruteforce your password!",
                &auth_server
            );

            *connection_state = ConnectionState::AuthTrustPrompt {
                auth_server,
                msg,
                status: connection_state.take_status_string(),
            };
        }
    }

    fn connection_error(&mut self, error: String) {
        if matches!(&self.screen, Screen::Connecting {..}) {
            self.screen = Screen::Login {
                screen: login::Screen::new(),
                error: Some(error),
            }
        }
    }
}

widget_ids! {
    struct Ids {
        // Background and logo
        bg,
        v_logo,
        alpha_version,
        alpha_text,
        banner,
        banner_top,
        // Disclaimer
        //disc_window,
        //disc_text_1,
        //disc_text_2,
        //disc_button,
        //disc_scrollbar,
        // Login, Singleplayer
        login_button,
        login_text,
        login_error,
        login_error_bg,
        address_text,
        address_bg,
        address_field,
        username_text,
        username_bg,
        username_field,
        password_text,
        password_bg,
        password_field,
        singleplayer_button,
        singleplayer_text,
        usrnm_bg,
        srvr_bg,
        passwd_bg,
        // Server list
        servers_button,
        servers_frame,
        servers_text,
        servers_close,
        // Buttons
        settings_button,
        quit_button,
        // Error
        error_frame,
        button_ok,
        version,
        // Info Window
        info_frame,
        info_text,
        info_bottom,
        // Auth Trust Prompt
        button_add_auth_trust,
    }
}

image_ids! {
    struct Imgs {
        <VoxelGraphic>
        v_logo: "voxygen.element.v_logo",

        info_frame: "voxygen.element.frames.info_frame_2",

        <ImageGraphic>
        bg: "voxygen.background.bg_main",
        banner_top: "voxygen.element.frames.banner_top",
        banner: "voxygen.element.frames.banner",
        banner_bottom: "voxygen.element.frames.banner_bottom",
        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",
        input_bg: "voxygen.element.misc_bg.textbox",
        //disclaimer: "voxygen.element.frames.disclaimer",


        <BlankGraphic>
        nothing: (),
    }
}

pub struct MainMenuUi {
    ui: Ui,
    ice_ui: IcedUi,
    ice_state: IcedState,
    ids: Ids,
    imgs: Imgs,
    username: String,
    password: String,
    server_address: String,
    popup: Option<PopupData>,
    connecting: Option<std::time::Instant>,
    connect: bool,
    show_servers: bool,
    //show_disclaimer: bool,
    time: f32,
    bg_img_id: conrod_core::image::Id,
    i18n: std::sync::Arc<Localization>,
    fonts: Fonts,
    pub show_iced: bool,
}

impl MainMenuUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        let window = &mut global_state.window;
        let networking = &global_state.settings.networking;
        let gameplay = &global_state.settings.gameplay;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(gameplay.ui_scale);
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::load(&mut ui).expect("Failed to load images");
        let bg_img_spec = BG_IMGS.choose(&mut thread_rng()).unwrap();
        let bg_img_id = ui.add_graphic(Graphic::Image(load_expect(bg_img_spec)));
        // Load language
        let i18n = load_expect::<Localization>(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ));
        // Load fonts.
        let fonts = Fonts::load(&i18n.fonts, &mut ui).expect("Impossible to load fonts!");

        // TODO: don't add default font twice
        let ice_font = {
            use std::io::Read;
            let mut buf = Vec::new();
            common::assets::load_file("voxygen.font.haxrcorp_4089_cyrillic_altgr_extended", &[
                "ttf",
            ])
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
            ui::ice::Font::try_from_vec(buf).unwrap()
        };

        let mut ice_ui = IcedUi::new(window, ice_font).unwrap();

        let ice_fonts =
            IcedFonts::load(&i18n.fonts, &mut ice_ui).expect("Impossible to load fonts");

        let ice_state = IcedState::new(
            ice_fonts,
            IcedImgs::load(&mut ice_ui).expect("Failed to load images"),
            ice_ui.add_graphic(Graphic::Image(load_expect(bg_img_spec))),
            i18n.clone(),
            &global_state.settings,
        );

        Self {
            ui,
            ice_ui,
            ice_state,
            ids,
            imgs,
            username: networking.username.clone(),
            password: "".to_owned(),
            server_address: networking.default_server.clone(),
            popup: None,
            connecting: None,
            show_servers: false,
            connect: false,
            time: 0.0,
            //show_disclaimer: global_state.settings.show_disclaimer,
            bg_img_id,
            i18n,
            fonts,
            show_iced: false,
        }
    }

    #[allow(clippy::assign_op_pattern)] // TODO: Pending review in #587
    #[allow(clippy::op_ref)] // TODO: Pending review in #587
    #[allow(clippy::toplevel_ref_arg)] // TODO: Pending review in #587
    fn update_layout(&mut self, global_state: &mut GlobalState, dt: Duration) -> Vec<Event> {
        let mut events = Vec::new();
        self.time = self.time + dt.as_secs_f32();
        let fade_msg = (self.time * 2.0).sin() * 0.5 + 0.51;
        let (ref mut ui_widgets, ref mut _tooltip_manager) = self.ui.set_widgets();
        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );
        const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
        const TEXT_COLOR_2: Color = Color::Rgba(1.0, 1.0, 1.0, 0.2);
        //const INACTIVE: Color = Color::Rgba(0.47, 0.47, 0.47, 0.47);

        let intro_text = &self.i18n.get("main.login_process");

        // Background image, Veloren logo, Alpha-Version Label
        Image::new(if self.connect {
            self.bg_img_id
        } else {
            self.imgs.bg
        })
        .middle_of(ui_widgets.window)
        .set(self.ids.bg, ui_widgets);

        // Version displayed top right corner
        Text::new(&version)
            .color(TEXT_COLOR)
            .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.version, ui_widgets);
        // Alpha Disclaimer
        Text::new(&format!("Veloren Pre-Alpha {}", env!("CARGO_PKG_VERSION")))
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(10))
            .color(TEXT_COLOR)
            .mid_top_with_margin_on(ui_widgets.window, 2.0)
            .set(self.ids.alpha_text, ui_widgets);
        // Popup (Error/Info/AuthTrustPrompt)
        let mut change_popup = None;
        if let Some(PopupData { msg, popup_type }) = &self.popup {
            let text = Text::new(msg)
                .rgba(
                    1.0,
                    1.0,
                    1.0,
                    if let PopupType::ConnectionInfo = popup_type {
                        fade_msg
                    } else {
                        1.0
                    },
                )
                .font_id(self.fonts.cyri.conrod_id);
            let (frame_w, frame_h) = if let PopupType::AuthTrustPrompt(_) = popup_type {
                (65.0 * 8.0, 370.0)
            } else {
                (65.0 * 6.0, 140.0)
            };
            let error_bg = Rectangle::fill_with([frame_w, frame_h], color::TRANSPARENT)
                .rgba(0.1, 0.1, 0.1, if self.connect { 0.0 } else { 1.0 })
                .parent(ui_widgets.window);
            if let PopupType::AuthTrustPrompt(_) = popup_type {
                error_bg.middle_of(ui_widgets.window)
            } else {
                error_bg.up_from(self.ids.banner_top, 15.0)
            }
            .set(self.ids.login_error_bg, ui_widgets);
            Image::new(self.imgs.info_frame)
                .w_h(frame_w, frame_h)
                .color(Some(Color::Rgba(
                    1.0,
                    1.0,
                    1.0,
                    if let PopupType::ConnectionInfo = popup_type {
                        0.0
                    } else {
                        1.0
                    },
                )))
                .middle_of(self.ids.login_error_bg)
                .set(self.ids.error_frame, ui_widgets);
            if let PopupType::ConnectionInfo = popup_type {
                text.mid_top_with_margin_on(self.ids.error_frame, 10.0)
                    .font_id(self.fonts.alkhemi.conrod_id)
                    .bottom_left_with_margins_on(ui_widgets.window, 60.0, 60.0)
                    .font_size(self.fonts.cyri.scale(70))
                    .set(self.ids.login_error, ui_widgets);
            } else {
                text.mid_top_with_margin_on(self.ids.error_frame, 10.0)
                    .w(frame_w - 10.0 * 2.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(25))
                    .set(self.ids.login_error, ui_widgets);
            };
            if Button::image(self.imgs.button)
                .w_h(100.0, 30.0)
                .mid_bottom_with_margin_on(
                    if let PopupType::ConnectionInfo = popup_type {
                        ui_widgets.window
                    } else {
                        self.ids.login_error_bg
                    },
                    10.0,
                )
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label_y(Relative::Scalar(2.0))
                .label(match popup_type {
                    PopupType::Error => self.i18n.get("common.okay"),
                    PopupType::ConnectionInfo => self.i18n.get("common.cancel"),
                    PopupType::AuthTrustPrompt(_) => self.i18n.get("common.cancel"),
                })
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))
                .label_color(TEXT_COLOR)
                .set(self.ids.button_ok, ui_widgets)
                .was_clicked()
            {
                match &popup_type {
                    PopupType::Error => (),
                    PopupType::ConnectionInfo => {
                        events.push(Event::CancelLoginAttempt);
                    },
                    PopupType::AuthTrustPrompt(auth_server) => {
                        events.push(Event::AuthServerTrust(auth_server.clone(), false));
                    },
                };
                change_popup = Some(None);
            }

            if let PopupType::AuthTrustPrompt(auth_server) = popup_type {
                if Button::image(self.imgs.button)
                    .w_h(100.0, 30.0)
                    .right_from(self.ids.button_ok, 10.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label("Add") // TODO: localize
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(15))
                    .label_color(TEXT_COLOR)
                    .set(self.ids.button_add_auth_trust, ui_widgets)
                    .was_clicked()
                {
                    events.push(Event::AuthServerTrust(auth_server.clone(), true));
                    change_popup = Some(Some(PopupData {
                        msg: self.i18n.get("main.connecting").into(),
                        popup_type: PopupType::ConnectionInfo,
                    }));
                }
            }
        }
        if let Some(p) = change_popup {
            self.popup = p;
        }

        if !self.connect {
            Image::new(self.imgs.banner)
                .w_h(65.0 * 6.0, 100.0 * 6.0)
                .middle_of(self.ids.bg)
                .color(Some(Color::Rgba(0.0, 0.0, 0.0, 0.9)))
                .set(self.ids.banner, ui_widgets);

            Image::new(self.imgs.banner_top)
                .w_h(70.0 * 6.0, 34.0)
                .mid_top_with_margin_on(self.ids.banner, -34.0)
                .set(self.ids.banner_top, ui_widgets);

            // Logo
            Image::new(self.imgs.v_logo)
                .w_h(123.0 * 2.5, 35.0 * 2.5)
                .mid_top_with_margin_on(self.ids.banner_top, 45.0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.95)))
                .set(self.ids.v_logo, ui_widgets);

            /*if self.show_disclaimer {
                Image::new(self.imgs.disclaimer)
                    .w_h(1800.0, 800.0)
                    .middle_of(ui_widgets.window)
                    .scroll_kids()
                    .scroll_kids_vertically()
                    .set(self.ids.disc_window, ui_widgets);

                Text::new(&self.i18n.get("common.disclaimer"))
                    .top_left_with_margins_on(self.ids.disc_window, 30.0, 40.0)
                    .font_size(self.fonts.cyri.scale(35))
                    .font_id(self.fonts.alkhemi.conrod_id)
                    .color(TEXT_COLOR)
                    .set(self.ids.disc_text_1, ui_widgets);
                Text::new(&self.i18n.get("main.notice"))
                    .top_left_with_margins_on(self.ids.disc_window, 110.0, 40.0)
                    .font_size(self.fonts.cyri.scale(26))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(self.ids.disc_text_2, ui_widgets);
                if Button::image(self.imgs.button)
                    .w_h(300.0, 50.0)
                    .mid_bottom_with_margin_on(self.ids.disc_window, 30.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label(&self.i18n.get("common.accept"))
                    .label_font_size(self.fonts.cyri.scale(22))
                    .label_color(TEXT_COLOR)
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .set(self.ids.disc_button, ui_widgets)
                    .was_clicked()
                {
                    self.show_disclaimer = false;
                    events.push(Event::DisclaimerAccepted);
                }
            } else {*/
            // TODO: Don't use macros for this?
            // Input fields
            // Used when the login button is pressed, or enter is pressed within input field
            macro_rules! login {
                () => {
                    self.connect = true;
                    self.connecting = Some(std::time::Instant::now());
                    self.popup = Some(PopupData {
                        msg: [self.i18n.get("main.connecting"), "..."].concat(),
                        popup_type: PopupType::ConnectionInfo,
                    });

                    events.push(Event::LoginAttempt {
                        username: self.username.clone(),
                        password: self.password.clone(),
                        server_address: self.server_address.clone(),
                    });
                };
            }
            // Info Window
            Rectangle::fill_with([550.0, 250.0], COL1)
                .top_left_with_margins_on(ui_widgets.window, 40.0, 40.0)
                .color(Color::Rgba(0.0, 0.0, 0.0, 0.95))
                .set(self.ids.info_frame, ui_widgets);
            Image::new(self.imgs.banner_bottom)
                .mid_bottom_with_margin_on(self.ids.info_frame, -50.0)
                .w_h(550.0, 50.0)
                .color(Some(Color::Rgba(0.0, 0.0, 0.0, 0.95)))
                .set(self.ids.info_bottom, ui_widgets);
            Text::new(intro_text)
                .top_left_with_margins_on(self.ids.info_frame, 15.0, 15.0)
                .font_size(self.fonts.cyri.scale(20))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(self.ids.info_text, ui_widgets);

            // Singleplayer
            // Used when the singleplayer button is pressed
            #[cfg(feature = "singleplayer")]
            macro_rules! singleplayer {
                () => {
                    events.push(Event::StartSingleplayer);
                    self.connect = true;
                    self.connecting = Some(std::time::Instant::now());
                    self.popup = Some(PopupData {
                        msg: [self.i18n.get("main.creating_world"), "..."].concat(),
                        popup_type: PopupType::ConnectionInfo,
                    });
                };
            }

            // Username
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.0))
                .mid_top_with_margin_on(self.ids.banner_top, 150.0)
                .set(self.ids.usrnm_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(338.0, 50.0)
                .middle_of(self.ids.usrnm_bg)
                .set(self.ids.username_bg, ui_widgets);
            for event in TextBox::new(&self.username)
                    .w_h(290.0, 30.0)
                    .mid_bottom_with_margin_on(self.ids.username_bg, 14.0)
                    .font_size(self.fonts.cyri.scale(22))
                    .font_id(self.fonts.cyri.conrod_id)
                    .text_color(TEXT_COLOR)
                    // transparent background
                    .color(TRANSPARENT)
                    .border_color(TRANSPARENT)
                    .set(self.ids.username_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(username) => {
                        // Note: TextBox limits the input string length to what fits in it
                        self.username = username.to_string();
                    },
                    TextBoxEvent::Enter => {
                        login!();
                    },
                }
            }
            // Password
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.0))
                .down_from(self.ids.usrnm_bg, 10.0)
                .set(self.ids.passwd_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(338.0, 50.0)
                .middle_of(self.ids.passwd_bg)
                .set(self.ids.password_bg, ui_widgets);
            for event in TextBox::new(&self.password)
                    .w_h(290.0, 30.0)
                    .mid_bottom_with_margin_on(self.ids.password_bg, 10.0)
                    // the text is smaller to allow longer passwords, conrod limits text length
                    // this allows 35 characters but can be increased, approximate formula: 420 / scale = length
                    .font_size(self.fonts.cyri.scale(12))
                    .font_id(self.fonts.cyri.conrod_id)
                    .text_color(TEXT_COLOR)
                    // transparent background
                    .color(TRANSPARENT)
                    .border_color(TRANSPARENT)
                    .hide_text("*")
                    .set(self.ids.password_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(password) => {
                        // Note: TextBox limits the input string length to what fits in it
                        self.password = password;
                    },
                    TextBoxEvent::Enter => {
                        self.password.pop();
                        login!();
                    },
                }
            }

            if self.show_servers {
                Image::new(self.imgs.info_frame)
                    .mid_top_with_margin_on(self.ids.username_bg, -320.0)
                    .w_h(400.0, 300.0)
                    .set(self.ids.servers_frame, ui_widgets);

                let ref mut net_settings = global_state.settings.networking;

                // TODO: Draw scroll bar or remove it.
                let (mut items, _scrollbar) = List::flow_down(net_settings.servers.len())
                    .top_left_with_margins_on(self.ids.servers_frame, 0.0, 5.0)
                    .w_h(400.0, 300.0)
                    .scrollbar_next_to()
                    .scrollbar_thickness(18.0)
                    .scrollbar_color(TEXT_COLOR)
                    .set(self.ids.servers_text, ui_widgets);

                while let Some(item) = items.next(ui_widgets) {
                    let mut text = "".to_string();
                    if &net_settings.servers[item.i] == &self.server_address {
                        text.push_str("-> ")
                    } else {
                        text.push_str("  ")
                    }
                    text.push_str(&net_settings.servers[item.i]);

                    if item
                        .set(
                            Button::image(self.imgs.nothing)
                                    .w_h(100.0, 50.0)
                                    .mid_top_with_margin_on(self.ids.servers_frame, 10.0)
                                    //.hover_image(self.imgs.button_hover)
                                    //.press_image(self.imgs.button_press)
                                    .label_y(Relative::Scalar(2.0))
                                    .label(&text)
                                    .label_font_size(self.fonts.cyri.scale(20))
                                    .label_font_id(self.fonts.cyri.conrod_id)
                                    .label_color(TEXT_COLOR),
                            ui_widgets,
                        )
                        .was_clicked()
                    {
                        self.server_address = net_settings.servers[item.i].clone();
                        net_settings.default_server = self.server_address.clone();
                    }
                }

                if Button::image(self.imgs.button)
                    .w_h(200.0, 53.0)
                    .mid_bottom_with_margin_on(self.ids.servers_frame, 5.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label(&self.i18n.get("common.close"))
                    .label_font_size(self.fonts.cyri.scale(20))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR)
                    .set(self.ids.servers_close, ui_widgets)
                    .was_clicked()
                {
                    self.show_servers = false
                };
            }
            // Server address
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.0))
                .down_from(self.ids.passwd_bg, 8.0)
                .set(self.ids.srvr_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(338.0, 50.0)
                .middle_of(self.ids.srvr_bg)
                .set(self.ids.address_bg, ui_widgets);
            for event in TextBox::new(&self.server_address)
                    .w_h(290.0, 30.0)
                    .mid_top_with_margin_on(self.ids.address_bg, 8.0)
                    .font_size(self.fonts.cyri.scale(22))
                    .font_id(self.fonts.cyri.conrod_id)
                    .text_color(TEXT_COLOR)
                    // transparent background
                    .color(TRANSPARENT)
                    .border_color(TRANSPARENT)
                    .set(self.ids.address_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(server_address) => {
                        self.server_address = server_address.to_string();
                    },
                    TextBoxEvent::Enter => {
                        login!();
                    },
                }
            }

            // Login button
            if Button::image(self.imgs.button)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .w_h(258.0, 55.0)
                    .down_from(self.ids.address_bg, 20.0)
                    .align_middle_x_of(self.ids.address_bg)
                    .label(&self.i18n.get("common.multiplayer"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(22))
                    .label_y(Relative::Scalar(5.0))
                    /*.with_tooltip(
                        tooltip_manager,
                        "Login",
                        "Click to login with the entered details",
                        &tooltip,
                    )
                    .tooltip_image(self.imgs.v_logo)*/
                    .set(self.ids.login_button, ui_widgets)
                    .was_clicked()
            {
                login!();
            }

            // Singleplayer button
            #[cfg(feature = "singleplayer")]
            {
                if Button::image(self.imgs.button)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .w_h(258.0, 55.0)
                    .down_from(self.ids.login_button, 20.0)
                    .align_middle_x_of(self.ids.address_bg)
                    .label(&self.i18n.get("common.singleplayer"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(22))
                    .label_y(Relative::Scalar(5.0))
                    .label_x(Relative::Scalar(2.0))
                    .set(self.ids.singleplayer_button, ui_widgets)
                    .was_clicked()
                {
                    singleplayer!();
                }
            }
            // Quit
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .bottom_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&self.i18n.get("common.quit"))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .label_font_size(self.fonts.cyri.scale(20))
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.quit_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Quit);
            }

            // Settings
            if Button::image(self.imgs.button)
                    .w_h(190.0, 40.0)
                    .up_from(self.ids.quit_button, 8.0)
                    //.hover_image(self.imgs.button_hover)
                    //.press_image(self.imgs.button_press)
                    .label(&self.i18n.get("common.settings"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR_2)
                    .label_font_size(self.fonts.cyri.scale(20))
                    .label_y(Relative::Scalar(3.0))
                    .set(self.ids.settings_button, ui_widgets)
                    .was_clicked()
            {
                events.push(Event::Settings);
            }

            // Servers
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .up_from(self.ids.settings_button, 8.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&self.i18n.get("common.servers"))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .label_font_size(self.fonts.cyri.scale(20))
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.servers_button, ui_widgets)
                .was_clicked()
            {
                self.show_servers = !self.show_servers;
            };
        }

        events
    }

    pub fn auth_trust_prompt(&mut self, auth_server: String) {
        self.ice_state.auth_trust_prompt(auth_server.clone());
        self.popup = Some(PopupData {
            msg: format!(
                "Warning: The server you are trying to connect to has provided this \
                 authentication server address:\n\n{}\n\nbut it is not in your list of trusted \
                 authentication servers.\n\nMake sure that you trust this site and owner to not \
                 try and bruteforce your password!",
                &auth_server
            ),
            popup_type: PopupType::AuthTrustPrompt(auth_server),
        })
    }

    pub fn show_info(&mut self, msg: String) {
        self.ice_state.connection_error(msg.clone());

        self.popup = Some(PopupData {
            msg,
            popup_type: PopupType::Error,
        });
        self.connecting = None;
        self.connect = false;
    }

    pub fn connected(&mut self) {
        self.popup = None;
        self.connecting = None;
        self.connect = false;
        self.ice_state.exit_connect_screen();
    }

    pub fn cancel_connection(&mut self) {
        self.popup = None;
        self.connecting = None;
        self.connect = false;
        self.ice_state.exit_connect_screen();
    }

    pub fn handle_event(&mut self, event: ui::Event) {
        if !self.show_iced {
            self.ui.handle_event(event);
        }
    }

    pub fn handle_iced_event(&mut self, event: ui::ice::Event) {
        if self.show_iced {
            self.ice_ui.handle_event(event);
        }
    }

    pub fn maintain(&mut self, global_state: &mut GlobalState, dt: Duration) -> Vec<Event> {
        let mut events = self.update_layout(global_state, dt);
        self.ui.maintain(global_state.window.renderer_mut(), None);
        if self.show_iced {
            let (messages, _) = self.ice_ui.maintain(
                self.ice_state.view(dt.as_secs_f32()),
                global_state.window.renderer_mut(),
            );
            messages
                .into_iter()
                .for_each(|message| self.ice_state.update(message, &mut events));
        }

        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer, None);
        if self.show_iced {
            self.ice_ui.render(renderer);
        }
    }
}
