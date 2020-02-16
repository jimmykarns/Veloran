use crate::{
    render::{Renderer, WinColorFmt, WinDepthFmt},
    settings::Settings,
    ui, Error,
};
use hashbrown::HashMap;
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use vek::*;

/// Represents a key that the game recognises after keyboard mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum GameInput {
    Primary,
    Secondary,
    ToggleCursor,
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    Jump,
    Sit,
    Glide,
    Climb,
    ClimbDown,
    WallLeap,
    Mount,
    Enter,
    Command,
    Escape,
    Map,
    Bag,
    QuestLog,
    CharacterWindow,
    Social,
    Spellbook,
    Settings,
    ToggleInterface,
    Help,
    ToggleDebug,
    Fullscreen,
    Screenshot,
    ToggleIngameUi,
    Roll,
    Respawn,
    Interact,
    ToggleWield,
    Charge,
    PlaceVox,
}

/// Represents an incoming event from the window.
#[derive(Clone)]
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// The window has been resized.
    Resize(Vec2<u32>),
    /// A key has been typed that corresponds to a specific character.
    Char(char),
    /// The cursor has been panned across the screen while grabbed.
    CursorPan(Vec2<f32>),
    /// The cursor has been moved across the screen while ungrabbed.
    CursorMove(Vec2<f32>),
    /// A mouse button has been pressed or released
    MouseButton(MouseButton, PressState),
    /// The camera has been requested to zoom.
    Zoom(f32),
    /// A key that the game recognises has been pressed or released.
    InputUpdate(GameInput, bool),
    /// Event that the ui uses.
    Ui(ui::Event),
    /// The view distance has changed.
    ViewDistanceChanged(u32),
    /// Game settings have changed.
    SettingsChanged,
    /// The window is (un)focused
    Focused(bool),
}

pub type MouseButton = winit::MouseButton;
pub type PressState = winit::ElementState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum KeyMouse {
    Key(glutin::VirtualKeyCode),
    Mouse(glutin::MouseButton),
}

impl fmt::Display for KeyMouse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::KeyMouse::*;
        use glutin::{MouseButton, VirtualKeyCode::*};
        write!(f, "{}", match self {
            Key(Key1) => "1",
            Key(Key2) => "2",
            Key(Key3) => "3",
            Key(Key4) => "4",
            Key(Key5) => "5",
            Key(Key6) => "6",
            Key(Key7) => "7",
            Key(Key8) => "8",
            Key(Key9) => "9",
            Key(Key0) => "0",
            Key(A) => "A",
            Key(B) => "B",
            Key(C) => "C",
            Key(D) => "D",
            Key(E) => "E",
            Key(F) => "F",
            Key(G) => "G",
            Key(H) => "H",
            Key(I) => "I",
            Key(J) => "J",
            Key(K) => "K",
            Key(L) => "L",
            Key(M) => "M",
            Key(N) => "N",
            Key(O) => "O",
            Key(P) => "P",
            Key(Q) => "Q",
            Key(R) => "R",
            Key(S) => "S",
            Key(T) => "T",
            Key(U) => "U",
            Key(V) => "V",
            Key(W) => "W",
            Key(X) => "X",
            Key(Y) => "Y",
            Key(Z) => "Z",
            Key(Escape) => "ESC",
            Key(F1) => "F1",
            Key(F2) => "F2",
            Key(F3) => "F3",
            Key(F4) => "F4",
            Key(F5) => "F5",
            Key(F6) => "F6",
            Key(F7) => "F7",
            Key(F8) => "F8",
            Key(F9) => "F9",
            Key(F10) => "F10",
            Key(F11) => "F11",
            Key(F12) => "F12",
            Key(F13) => "F13",
            Key(F14) => "F14",
            Key(F15) => "F15",
            Key(F16) => "F16",
            Key(F17) => "F17",
            Key(F18) => "F18",
            Key(F19) => "F19",
            Key(F20) => "F20",
            Key(F21) => "F21",
            Key(F22) => "F22",
            Key(F23) => "F23",
            Key(F24) => "F24",
            Key(Snapshot) => "Print Screen",
            Key(Scroll) => "Scroll Lock",
            Key(Pause) => "Pause/Break",
            Key(Insert) => "Insert",
            Key(Home) => "Home",
            Key(Delete) => "Delete",
            Key(End) => "End",
            Key(PageDown) => "PageDown",
            Key(PageUp) => "PageUp",
            Key(Left) => "Left Arrow",
            Key(Up) => "Up Arrow",
            Key(Right) => "Right Arrow",
            Key(Down) => "Down Arrow",
            Key(Back) => "Backspace",
            Key(Return) => "Enter",
            Key(Space) => "Space",
            Key(Compose) => "Compose",
            Key(Caret) => "^",
            Key(Numlock) => "Numlock",
            Key(Numpad0) => "Numpad 0",
            Key(Numpad1) => "Numpad 1",
            Key(Numpad2) => "Numpad 2",
            Key(Numpad3) => "Numpad 3",
            Key(Numpad4) => "Numpad 4",
            Key(Numpad5) => "Numpad 5",
            Key(Numpad6) => "Numpad 6",
            Key(Numpad7) => "Numpad 7",
            Key(Numpad8) => "Numpad 8",
            Key(Numpad9) => "Numpad 9",
            Key(AbntC1) => "Abnt C1",
            Key(AbntC2) => "Abnt C2",
            Key(Add) => "Numpad +",
            Key(Apostrophe) => "'",
            Key(Apps) => "Context Menu",
            Key(At) => "@",
            Key(Ax) => "Ax",
            Key(Backslash) => "\\",
            Key(Calculator) => "Calculator",
            Key(Capital) => "Caps Lock",
            Key(Colon) => ":",
            Key(Comma) => ",",
            Key(Convert) => "Convert",
            Key(Decimal) => "Numpad .",
            Key(Divide) => "Numpad /",
            Key(Equals) => "=",
            Key(Grave) => "`",
            Key(Kana) => "Kana",
            Key(Kanji) => "Kanji",
            Key(LAlt) => "LAlt",
            Key(LBracket) => "[",
            Key(LControl) => "LControl",
            Key(LShift) => "LShift",
            Key(LWin) => "LWin",
            Key(Mail) => "Mail",
            Key(MediaSelect) => "MediaSelect",
            Key(MediaStop) => "MediaStop",
            Key(Minus) => "-",
            Key(Multiply) => "Numpad *",
            Key(Mute) => "Mute",
            Key(MyComputer) => "My Computer",
            Key(NavigateForward) => "Navigate Forward",
            Key(NavigateBackward) => "Navigate Backward",
            Key(NextTrack) => "Next Track",
            Key(NoConvert) => "Non Convert",
            Key(NumpadComma) => "Numpad ,",
            Key(NumpadEnter) => "Numpad Enter",
            Key(NumpadEquals) => "Numpad =",
            Key(OEM102) => "OEM 102",
            Key(Period) => ".",
            Key(PlayPause) => "Play / Pause",
            Key(Power) => "Power",
            Key(PrevTrack) => "Prev Track",
            Key(RAlt) => "RAlt",
            Key(RBracket) => "]",
            Key(RControl) => "RControl",
            Key(RShift) => "RShift",
            Key(RWin) => "RWin",
            Key(Semicolon) => ";",
            Key(Slash) => "/",
            Key(Sleep) => "Sleep",
            Key(Stop) => "Media Stop",
            Key(Subtract) => "Numpad -",
            Key(Sysrq) => "Sysrq",
            Key(Tab) => "Tab",
            Key(Underline) => "_",
            Key(Unlabeled) => "No Name",
            Key(VolumeDown) => "Volume Down",
            Key(VolumeUp) => "Volume Up",
            Key(Wake) => "Wake",
            Key(WebBack) => "Browser Back",
            Key(WebFavorites) => "Browser Favorites",
            Key(WebForward) => "Browser Forward",
            Key(WebHome) => "Browser Home",
            Key(WebRefresh) => "Browser Refresh",
            Key(WebSearch) => "Browser Search",
            Key(WebStop) => "Browser Stop",
            Key(Yen) => "Yen",
            Key(Copy) => "Copy",
            Key(Paste) => "Paste",
            Key(Cut) => "Cut",
            Mouse(MouseButton::Left) => "Mouse L-Click",
            Mouse(MouseButton::Right) => "Mouse R-Click",
            Mouse(MouseButton::Middle) => "Mouse Middle-Click",
            Mouse(MouseButton::Other(button)) =>
                return write!(f, "Unknown Mouse Button: {:?}", button),
        })
    }
}

pub struct Window {
    events_loop: glutin::EventsLoop,
    renderer: Renderer,
    window: glutin::ContextWrapper<glutin::PossiblyCurrent, winit::Window>,
    cursor_grabbed: bool,
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub zoom_inversion: bool,
    pub mouse_y_inversion: bool,
    fullscreen: bool,
    needs_refresh_resize: bool,
    key_map: HashMap<KeyMouse, Vec<GameInput>>,
    keypress_map: HashMap<GameInput, glutin::ElementState>,
    supplement_events: Vec<Event>,
    focused: bool,
}

impl Window {
    pub fn new(settings: &Settings) -> Result<Window, Error> {
        let events_loop = glutin::EventsLoop::new();

        let size = settings.graphics.window_size;

        let win_builder = glutin::WindowBuilder::new()
            .with_title("Veloren")
            .with_dimensions(glutin::dpi::LogicalSize::new(
                size[0] as f64,
                size[1] as f64,
            ))
            .with_maximized(true);

        let ctx_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
            .with_vsync(false);

        let (window, device, factory, win_color_view, win_depth_view) =
            gfx_window_glutin::init::<WinColorFmt, WinDepthFmt>(
                win_builder,
                ctx_builder,
                &events_loop,
            )
            .map_err(|err| Error::BackendError(Box::new(err)))?;

        let mut map: HashMap<_, Vec<_>> = HashMap::new();
        map.entry(settings.controls.primary)
            .or_default()
            .push(GameInput::Primary);
        map.entry(settings.controls.secondary)
            .or_default()
            .push(GameInput::Secondary);
        map.entry(settings.controls.toggle_cursor)
            .or_default()
            .push(GameInput::ToggleCursor);
        map.entry(settings.controls.escape)
            .or_default()
            .push(GameInput::Escape);
        map.entry(settings.controls.enter)
            .or_default()
            .push(GameInput::Enter);
        map.entry(settings.controls.command)
            .or_default()
            .push(GameInput::Command);
        map.entry(settings.controls.move_forward)
            .or_default()
            .push(GameInput::MoveForward);
        map.entry(settings.controls.move_left)
            .or_default()
            .push(GameInput::MoveLeft);
        map.entry(settings.controls.move_back)
            .or_default()
            .push(GameInput::MoveBack);
        map.entry(settings.controls.move_right)
            .or_default()
            .push(GameInput::MoveRight);
        map.entry(settings.controls.jump)
            .or_default()
            .push(GameInput::Jump);
        map.entry(settings.controls.sit)
            .or_default()
            .push(GameInput::Sit);
        map.entry(settings.controls.glide)
            .or_default()
            .push(GameInput::Glide);
        map.entry(settings.controls.climb)
            .or_default()
            .push(GameInput::Climb);
        map.entry(settings.controls.climb_down)
            .or_default()
            .push(GameInput::ClimbDown);
        map.entry(settings.controls.wall_leap)
            .or_default()
            .push(GameInput::WallLeap);
        map.entry(settings.controls.mount)
            .or_default()
            .push(GameInput::Mount);
        map.entry(settings.controls.map)
            .or_default()
            .push(GameInput::Map);
        map.entry(settings.controls.bag)
            .or_default()
            .push(GameInput::Bag);
        map.entry(settings.controls.quest_log)
            .or_default()
            .push(GameInput::QuestLog);
        map.entry(settings.controls.character_window)
            .or_default()
            .push(GameInput::CharacterWindow);
        map.entry(settings.controls.social)
            .or_default()
            .push(GameInput::Social);
        map.entry(settings.controls.spellbook)
            .or_default()
            .push(GameInput::Spellbook);
        map.entry(settings.controls.settings)
            .or_default()
            .push(GameInput::Settings);
        map.entry(settings.controls.help)
            .or_default()
            .push(GameInput::Help);
        map.entry(settings.controls.toggle_interface)
            .or_default()
            .push(GameInput::ToggleInterface);
        map.entry(settings.controls.toggle_debug)
            .or_default()
            .push(GameInput::ToggleDebug);
        map.entry(settings.controls.fullscreen)
            .or_default()
            .push(GameInput::Fullscreen);
        map.entry(settings.controls.screenshot)
            .or_default()
            .push(GameInput::Screenshot);
        map.entry(settings.controls.toggle_ingame_ui)
            .or_default()
            .push(GameInput::ToggleIngameUi);
        map.entry(settings.controls.roll)
            .or_default()
            .push(GameInput::Roll);
        map.entry(settings.controls.respawn)
            .or_default()
            .push(GameInput::Respawn);
        map.entry(settings.controls.interact)
            .or_default()
            .push(GameInput::Interact);
        map.entry(settings.controls.toggle_wield)
            .or_default()
            .push(GameInput::ToggleWield);
        map.entry(settings.controls.charge)
            .or_default()
            .push(GameInput::Charge);
        map.entry(settings.controls.place_vox)
            .or_default()
            .push(GameInput::PlaceVox);

        let keypress_map = HashMap::new();

        let mut this = Self {
            events_loop,
            renderer: Renderer::new(
                device,
                factory,
                win_color_view,
                win_depth_view,
                settings.graphics.aa_mode,
                settings.graphics.cloud_mode,
                settings.graphics.fluid_mode,
            )?,
            window,
            cursor_grabbed: false,
            pan_sensitivity: settings.gameplay.pan_sensitivity,
            zoom_sensitivity: settings.gameplay.zoom_sensitivity,
            zoom_inversion: settings.gameplay.zoom_inversion,
            mouse_y_inversion: settings.gameplay.mouse_y_inversion,
            fullscreen: false,
            needs_refresh_resize: false,
            key_map: map,
            keypress_map,
            supplement_events: vec![],
            focused: true,
        };

        this.fullscreen(settings.graphics.fullscreen);

        Ok(this)
    }

    pub fn renderer(&self) -> &Renderer { &self.renderer }

    pub fn renderer_mut(&mut self) -> &mut Renderer { &mut self.renderer }

    pub fn fetch_events(&mut self, settings: &mut Settings) -> Vec<Event> {
        let mut events = vec![];
        events.append(&mut self.supplement_events);
        // Refresh ui size (used when changing playstates)
        if self.needs_refresh_resize {
            events.push(Event::Ui(ui::Event::new_resize(self.logical_size())));
            self.needs_refresh_resize = false;
        }

        // Copy data that is needed by the events closure to avoid lifetime errors.
        // TODO: Remove this if/when the compiler permits it.
        let cursor_grabbed = self.cursor_grabbed;
        let renderer = &mut self.renderer;
        let window = &mut self.window;
        let focused = &mut self.focused;
        let key_map = &self.key_map;
        let keypress_map = &mut self.keypress_map;
        let pan_sensitivity = self.pan_sensitivity;
        let zoom_sensitivity = self.zoom_sensitivity;
        let zoom_inversion = match self.zoom_inversion {
            true => -1.0,
            false => 1.0,
        };
        let mouse_y_inversion = match self.mouse_y_inversion {
            true => -1.0,
            false => 1.0,
        };
        let mut toggle_fullscreen = false;
        let mut take_screenshot = false;

        self.events_loop.poll_events(|event| {
            // Get events for ui.
            if let Some(event) = ui::Event::try_from(event.clone(), window) {
                events.push(Event::Ui(event));
            }

            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => events.push(Event::Close),
                    glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                        let (mut color_view, mut depth_view) = renderer.win_views_mut();
                        gfx_window_glutin::update_views(window, &mut color_view, &mut depth_view);
                        renderer.on_resize().unwrap();
                        events.push(Event::Resize(Vec2::new(width as u32, height as u32)));
                    },
                    glutin::WindowEvent::ReceivedCharacter(c) => events.push(Event::Char(c)),
                    glutin::WindowEvent::MouseInput { button, state, .. } => {
                        if let (true, Some(game_inputs)) =
                            (cursor_grabbed, key_map.get(&KeyMouse::Mouse(button)))
                        {
                            for game_input in game_inputs {
                                events.push(Event::InputUpdate(
                                    *game_input,
                                    state == glutin::ElementState::Pressed,
                                ));
                            }
                        }
                        events.push(Event::MouseButton(button, state));
                    },
                    glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode
                    {
                        Some(key) => {
                            let game_inputs = key_map.get(&KeyMouse::Key(key));
                            if let Some(game_inputs) = game_inputs {
                                for game_input in game_inputs {
                                    match game_input {
                                        GameInput::Fullscreen => {
                                            if input.state == glutin::ElementState::Pressed
                                                && !Self::is_pressed(
                                                    keypress_map,
                                                    GameInput::Fullscreen,
                                                )
                                            {
                                                toggle_fullscreen = !toggle_fullscreen;
                                            }
                                            Self::set_pressed(
                                                keypress_map,
                                                GameInput::Fullscreen,
                                                input.state,
                                            );
                                        },
                                        GameInput::Screenshot => {
                                            take_screenshot = input.state
                                                == glutin::ElementState::Pressed
                                                && !Self::is_pressed(
                                                    keypress_map,
                                                    GameInput::Screenshot,
                                                );
                                            Self::set_pressed(
                                                keypress_map,
                                                GameInput::Screenshot,
                                                input.state,
                                            );
                                        },
                                        _ => events.push(Event::InputUpdate(
                                            *game_input,
                                            input.state == glutin::ElementState::Pressed,
                                        )),
                                    }
                                }
                            }
                        },
                        _ => {},
                    },
                    glutin::WindowEvent::Focused(state) => {
                        *focused = state;
                        events.push(Event::Focused(state));
                    },
                    _ => {},
                },
                glutin::Event::DeviceEvent { event, .. } => match event {
                    glutin::DeviceEvent::MouseMotion {
                        delta: (dx, dy), ..
                    } if *focused => {
                        let delta = Vec2::new(
                            dx as f32 * (pan_sensitivity as f32 / 100.0),
                            dy as f32 * (pan_sensitivity as f32 * mouse_y_inversion / 100.0),
                        );

                        if cursor_grabbed {
                            events.push(Event::CursorPan(delta));
                        } else {
                            events.push(Event::CursorMove(delta));
                        }
                    },
                    glutin::DeviceEvent::MouseWheel { delta, .. } if cursor_grabbed && *focused => {
                        events.push(Event::Zoom({
                            let y = match delta {
                                glutin::MouseScrollDelta::LineDelta(_x, y) => y,
                                // TODO: Check to see if there is a better way to find the "line
                                // height" than just hardcoding 16.0 pixels.  Alternately we could
                                // get rid of this and have the user set zoom sensitivity, since
                                // it's unlikely people would expect a configuration file to work
                                // across operating systems.
                                glutin::MouseScrollDelta::PixelDelta(pos) => (pos.y / 16.0) as f32,
                            };
                            y * (zoom_sensitivity as f32 / 100.0) * zoom_inversion
                        }))
                    },
                    _ => {},
                },
                _ => {},
            }
        });

        if take_screenshot {
            self.take_screenshot();
        }

        if toggle_fullscreen {
            self.toggle_fullscreen(settings);
        }

        events
    }

    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.window
            .swap_buffers()
            .map_err(|err| Error::BackendError(Box::new(err)))
    }

    pub fn is_cursor_grabbed(&self) -> bool { self.cursor_grabbed }

    pub fn grab_cursor(&mut self, grab: bool) {
        self.cursor_grabbed = grab;
        self.window.window().hide_cursor(grab);
        let _ = self.window.window().grab_cursor(grab);
    }

    pub fn toggle_fullscreen(&mut self, settings: &mut Settings) {
        self.fullscreen(!self.is_fullscreen());
        settings.graphics.fullscreen = self.is_fullscreen();
        settings.save_to_file_warn();
    }

    pub fn is_fullscreen(&self) -> bool { self.fullscreen }

    pub fn fullscreen(&mut self, fullscreen: bool) {
        let window = self.window.window();
        self.fullscreen = fullscreen;
        if fullscreen {
            window.set_fullscreen(Some(window.get_current_monitor()));
        } else {
            window.set_fullscreen(None);
        }
    }

    pub fn needs_refresh_resize(&mut self) { self.needs_refresh_resize = true; }

    pub fn logical_size(&self) -> Vec2<f64> {
        let (w, h) = self
            .window
            .window()
            .get_inner_size()
            .unwrap_or(glutin::dpi::LogicalSize::new(0.0, 0.0))
            .into();
        Vec2::new(w, h)
    }

    pub fn set_size(&mut self, new_size: Vec2<u16>) {
        self.window
            .window()
            .set_inner_size(glutin::dpi::LogicalSize::new(
                new_size.x as f64,
                new_size.y as f64,
            ));
    }

    pub fn send_supplement_event(&mut self, event: Event) { self.supplement_events.push(event) }

    pub fn take_screenshot(&mut self) {
        match self.renderer.create_screenshot() {
            Ok(img) => {
                std::thread::spawn(move || {
                    use std::{path::PathBuf, time::SystemTime};
                    // Check if folder exists and create it if it does not
                    let mut path = PathBuf::from("./screenshots");
                    if !path.exists() {
                        if let Err(err) = std::fs::create_dir(&path) {
                            warn!("Couldn't create folder for screenshot: {:?}", err);
                        }
                    }
                    path.push(format!(
                        "screenshot_{}.png",
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0)
                    ));
                    if let Err(err) = img.save(&path) {
                        warn!("Couldn't save screenshot: {:?}", err);
                    }
                });
            },
            Err(err) => error!(
                "Couldn't create screenshot due to renderer error: {:?}",
                err
            ),
        }
    }

    fn is_pressed(map: &mut HashMap<GameInput, glutin::ElementState>, input: GameInput) -> bool {
        *(map.entry(input).or_insert(glutin::ElementState::Released))
            == glutin::ElementState::Pressed
    }

    fn set_pressed(
        map: &mut HashMap<GameInput, glutin::ElementState>,
        input: GameInput,
        state: glutin::ElementState,
    ) {
        map.insert(input, state);
    }
}
