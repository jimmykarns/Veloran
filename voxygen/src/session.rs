use crate::{
    ecs::MyEntity,
    hud::{DebugInfo, Event as HudEvent, Hud, HudInfo, PressBehavior},
    i18n::{i18n_asset_key, Localization},
    key_state::KeyState,
    menu::char_selection::CharSelectionState,
    scene::{camera, Scene, SceneData},
    settings::AudioOutput,
    window::{AnalogGameInput, Event, GameInput},
    Direction, Error, GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client, Event::Chat};
use common::{
    assets::{load_watched, watch},
    clock::Clock,
    comp,
    comp::{Pos, Vel, MAX_PICKUP_RANGE_SQR},
    msg::{ClientState, Notification},
    terrain::{Block, BlockKind},
    util::Dir,
    vol::ReadVol,
    ChatType,
};
use specs::{Join, WorldExt};
use std::{cell::RefCell, rc::Rc, time::Duration};
use tracing::{error, info};
use vek::*;

/// The action to perform after a tick
enum TickAction {
    // Continue executing
    Continue,
    // Disconnected (i.e. go to main menu)
    Disconnect,
}

pub struct SessionState {
    scene: Scene,
    client: Rc<RefCell<Client>>,
    hud: Hud,
    key_state: KeyState,
    inputs: comp::ControllerInputs,
    selected_block: Block,
}

/// Represents an active game session (i.e., the one being played).
impl SessionState {
    /// Create a new `SessionState`.
    pub fn new(global_state: &mut GlobalState, client: Rc<RefCell<Client>>) -> Self {
        // Create a scene for this session. The scene handles visible elements of the
        // game world.
        let mut scene = Scene::new(global_state.window.renderer_mut());
        scene
            .camera_mut()
            .set_fov_deg(global_state.settings.graphics.fov);
        let hud = Hud::new(global_state, &client.borrow());
        {
            let my_entity = client.borrow().entity();
            client
                .borrow_mut()
                .state_mut()
                .ecs_mut()
                .insert(MyEntity(my_entity));
        }
        Self {
            scene,
            client,
            key_state: KeyState::new(),
            inputs: comp::ControllerInputs::default(),
            hud,
            selected_block: Block::new(BlockKind::Normal, Rgb::broadcast(255)),
        }
    }
}

impl SessionState {
    /// Tick the session (and the client attached to it).
    fn tick(&mut self, dt: Duration, global_state: &mut GlobalState) -> Result<TickAction, Error> {
        self.inputs.tick(dt);

        for event in self.client.borrow_mut().tick(
            self.inputs.clone(),
            dt,
            crate::ecs::sys::add_local_systems,
        )? {
            match event {
                Chat {
                    chat_type: _,
                    ref message,
                } => {
                    info!("[CHAT] {}", message);
                    self.hud.new_message(event);
                },
                client::Event::Disconnect => return Ok(TickAction::Disconnect),
                client::Event::DisconnectionNotification(time) => {
                    let message = match time {
                        0 => String::from("Goodbye!"),
                        _ => format!("Connection lost. Kicking in {} seconds", time),
                    };

                    self.hud.new_message(Chat {
                        chat_type: ChatType::Meta,
                        message,
                    });
                },
                client::Event::Notification(Notification::WaypointSaved) => {
                    self.hud
                        .new_message(client::Event::Notification(Notification::WaypointSaved));
                },
                client::Event::SetViewDistance(vd) => {
                    global_state.settings.graphics.view_distance = vd;
                    global_state.settings.save_to_file_warn();
                },
            }
        }

        Ok(TickAction::Continue)
    }

    /// Clean up the session (and the client attached to it) after a tick.
    pub fn cleanup(&mut self) { self.client.borrow_mut().cleanup(); }
}

impl PlayState for SessionState {
    fn play(&mut self, _: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        // Trap the cursor.
        global_state.window.grab_cursor(true);

        // Set up an fps clock.
        let mut clock = Clock::start();
        self.client.borrow_mut().clear_terrain();

        // Send startup commands to the server
        if global_state.settings.send_logon_commands {
            for cmd in &global_state.settings.logon_commands {
                self.client.borrow_mut().send_chat(cmd.to_string());
            }
        }

        // Keep a watcher on the language
        let mut localization_watcher = watch::ReloadIndicator::new();
        let mut localized_strings = load_watched::<Localization>(
            &i18n_asset_key(&global_state.settings.language.selected_language),
            &mut localization_watcher,
        )
        .unwrap();

        let mut ori = self.scene.camera().get_orientation();
        let mut free_look = false;
        let mut auto_walk = false;

        fn stop_auto_walk(auto_walk: &mut bool, key_state: &mut KeyState, hud: &mut Hud) {
            *auto_walk = false;
            hud.auto_walk(false);
            key_state.auto_walk = false;
        }

        // Game loop
        let mut current_client_state = self.client.borrow().get_client_state();
        while let ClientState::Pending | ClientState::Character = current_client_state {
            // Compute camera data
            self.scene
                .camera_mut()
                .compute_dependents(&*self.client.borrow().state().terrain());
            let camera::Dependents {
                view_mat, cam_pos, ..
            } = self.scene.camera().dependents();

            // Choose a spot above the player's head for item distance checks
            let player_pos = match self
                .client
                .borrow()
                .state()
                .read_storage::<comp::Pos>()
                .get(self.client.borrow().entity())
            {
                Some(pos) => pos.0 + (Vec3::unit_z() * 2.0),
                _ => cam_pos, // Should never happen, but a safe fallback
            };

            let (is_aiming, aim_dir_offset) = {
                let client = self.client.borrow();
                let is_aiming = client
                    .state()
                    .read_storage::<comp::CharacterState>()
                    .get(client.entity())
                    .map(|cs| cs.is_aimed())
                    .unwrap_or(false);

                (
                    is_aiming,
                    if is_aiming {
                        Vec3::unit_z() * 0.025
                    } else {
                        Vec3::zero()
                    },
                )
            };

            let cam_dir: Vec3<f32> = Vec3::from(view_mat.inverted() * -Vec4::unit_z());

            // Check to see whether we're aiming at anything
            let (build_pos, select_pos) = {
                let client = self.client.borrow();
                let terrain = client.state().terrain();

                let cam_ray = terrain
                    .ray(cam_pos, cam_pos + cam_dir * 100.0)
                    .until(|block| block.is_tangible())
                    .cast();

                let cam_dist = cam_ray.0;

                match cam_ray.1 {
                    Ok(Some(_))
                        if player_pos.distance_squared(cam_pos + cam_dir * cam_dist)
                            <= MAX_PICKUP_RANGE_SQR =>
                    {
                        (
                            Some((cam_pos + cam_dir * (cam_dist - 0.01)).map(|e| e.floor() as i32)),
                            Some((cam_pos + cam_dir * cam_dist).map(|e| e.floor() as i32)),
                        )
                    },
                    _ => (None, None),
                }
            };

            // Only highlight collectables
            self.scene.set_select_pos(select_pos.filter(|sp| {
                self.client
                    .borrow()
                    .state()
                    .terrain()
                    .get(*sp)
                    .map(|b| b.is_collectible())
                    .unwrap_or(false)
            }));

            // Handle window events.
            for event in global_state.window.fetch_events(&mut global_state.settings) {
                // Pass all events to the ui first.
                if self.hud.handle_event(event.clone(), global_state) {
                    continue;
                }

                match event {
                    Event::Close => {
                        return PlayStateResult::Shutdown;
                    },
                    Event::InputUpdate(GameInput::Primary, state) => {
                        // Check the existence of CanBuild component. If it's here, use LMB to
                        // break blocks, if not, use it to attack
                        let mut client = self.client.borrow_mut();
                        if state
                            && client
                                .state()
                                .read_storage::<comp::CanBuild>()
                                .get(client.entity())
                                .is_some()
                        {
                            if let Some(select_pos) = select_pos {
                                client.remove_block(select_pos);
                            }
                        } else {
                            self.inputs.primary.set_state(state);
                        }
                    },

                    Event::InputUpdate(GameInput::Secondary, state) => {
                        self.inputs.secondary.set_state(false); // To be changed later on

                        let mut client = self.client.borrow_mut();

                        if state
                            && client
                                .state()
                                .read_storage::<comp::CanBuild>()
                                .get(client.entity())
                                .is_some()
                        {
                            if let Some(build_pos) = build_pos {
                                client.place_block(build_pos, self.selected_block);
                            }
                        } else {
                            self.inputs.secondary.set_state(state);
                        }
                    },

                    Event::InputUpdate(GameInput::Roll, state) => {
                        let client = self.client.borrow();
                        if client
                            .state()
                            .read_storage::<comp::CanBuild>()
                            .get(client.entity())
                            .is_some()
                        {
                            if state {
                                if let Some(block) = select_pos
                                    .and_then(|sp| client.state().terrain().get(sp).ok().copied())
                                {
                                    self.selected_block = block;
                                }
                            }
                        } else {
                            self.inputs.roll.set_state(state);
                        }
                    },
                    Event::InputUpdate(GameInput::Respawn, state)
                        if state != self.key_state.respawn =>
                    {
                        self.key_state.respawn = state;
                        if state {
                            self.client.borrow_mut().respawn();
                        }
                    }
                    Event::InputUpdate(GameInput::Jump, state) => {
                        self.inputs.jump.set_state(state);
                    },
                    Event::InputUpdate(GameInput::Sit, state)
                        if state != self.key_state.toggle_sit =>
                    {
                        self.key_state.toggle_sit = state;
                        if state {
                            stop_auto_walk(&mut auto_walk, &mut self.key_state, &mut self.hud);
                            self.client.borrow_mut().toggle_sit();
                        }
                    }
                    Event::InputUpdate(GameInput::Dance, state)
                        if state != self.key_state.toggle_dance =>
                    {
                        self.key_state.toggle_dance = state;
                        if state {
                            stop_auto_walk(&mut auto_walk, &mut self.key_state, &mut self.hud);
                            self.client.borrow_mut().toggle_dance();
                        }
                    }
                    Event::InputUpdate(GameInput::MoveForward, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            stop_auto_walk(&mut auto_walk, &mut self.key_state, &mut self.hud);
                        }
                        self.key_state.up = state
                    },
                    Event::InputUpdate(GameInput::MoveBack, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            stop_auto_walk(&mut auto_walk, &mut self.key_state, &mut self.hud);
                        }
                        self.key_state.down = state
                    },
                    Event::InputUpdate(GameInput::MoveLeft, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            stop_auto_walk(&mut auto_walk, &mut self.key_state, &mut self.hud);
                        }
                        self.key_state.left = state
                    },
                    Event::InputUpdate(GameInput::MoveRight, state) => {
                        if state && global_state.settings.gameplay.stop_auto_walk_on_input {
                            stop_auto_walk(&mut auto_walk, &mut self.key_state, &mut self.hud);
                        }
                        self.key_state.right = state
                    },
                    Event::InputUpdate(GameInput::Glide, state)
                        if state != self.key_state.toggle_glide =>
                    {
                        self.key_state.toggle_glide = state;
                        if state {
                            self.client.borrow_mut().toggle_glide();
                        }
                    }
                    Event::InputUpdate(GameInput::Climb, state) => {
                        self.key_state.climb_up = state;
                    },
                    Event::InputUpdate(GameInput::ClimbDown, state) => {
                        self.key_state.climb_down = state;
                    },
                    /*Event::InputUpdate(GameInput::WallLeap, state) => {
                        self.inputs.wall_leap.set_state(state)
                    },*/
                    Event::InputUpdate(GameInput::ToggleWield, state)
                        if state != self.key_state.toggle_wield =>
                    {
                        self.key_state.toggle_wield = state;
                        if state {
                            self.client.borrow_mut().toggle_wield();
                        }
                    }
                    Event::InputUpdate(GameInput::SwapLoadout, state)
                        if state != self.key_state.swap_loadout =>
                    {
                        self.key_state.swap_loadout = state;
                        if state {
                            self.client.borrow_mut().swap_loadout();
                        }
                    }
                    Event::InputUpdate(GameInput::ToggleLantern, true) => {
                        self.client.borrow_mut().toggle_lantern();
                    },
                    Event::InputUpdate(GameInput::Mount, true) => {
                        let mut client = self.client.borrow_mut();
                        if client.is_mounted() {
                            client.unmount();
                        } else {
                            let player_pos = client
                                .state()
                                .read_storage::<comp::Pos>()
                                .get(client.entity())
                                .copied();
                            if let Some(player_pos) = player_pos {
                                // Find closest mountable entity
                                let closest_mountable = (
                                    &client.state().ecs().entities(),
                                    &client.state().ecs().read_storage::<comp::Pos>(),
                                    &client.state().ecs().read_storage::<comp::MountState>(),
                                )
                                    .join()
                                    .filter(|(_, _, ms)| {
                                        if let comp::MountState::Unmounted = ms {
                                            true
                                        } else {
                                            false
                                        }
                                    })
                                    .min_by_key(|(_, pos, _)| {
                                        (player_pos.0.distance_squared(pos.0) * 1000.0) as i32
                                    })
                                    .map(|(uid, _, _)| uid);

                                if let Some(mountee_entity) = closest_mountable {
                                    client.mount(mountee_entity);
                                }
                            }
                        }
                    },
                    Event::InputUpdate(GameInput::Interact, state) => {
                        let mut client = self.client.borrow_mut();

                        // Collect terrain sprites
                        if let Some(select_pos) = self.scene.select_pos() {
                            client.collect_block(select_pos);
                        }

                        // Collect lootable entities
                        let player_pos = client
                            .state()
                            .read_storage::<comp::Pos>()
                            .get(client.entity())
                            .copied();

                        if let (Some(player_pos), true) = (player_pos, state) {
                            let entity = (
                                &client.state().ecs().entities(),
                                &client.state().ecs().read_storage::<comp::Pos>(),
                                &client.state().ecs().read_storage::<comp::Item>(),
                            )
                                .join()
                                .filter(|(_, pos, _)| {
                                    pos.0.distance_squared(player_pos.0) < MAX_PICKUP_RANGE_SQR
                                })
                                .min_by_key(|(_, pos, _)| {
                                    (pos.0.distance_squared(player_pos.0) * 1000.0) as i32
                                })
                                .map(|(entity, _, _)| entity);

                            if let Some(entity) = entity {
                                client.pick_up(entity);
                            }
                        }
                    },
                    /*Event::InputUpdate(GameInput::Charge, state) => {
                        self.inputs.charge.set_state(state);
                    },*/
                    Event::InputUpdate(GameInput::FreeLook, state) => {
                        match (global_state.settings.gameplay.free_look_behavior, state) {
                            (PressBehavior::Toggle, true) => {
                                free_look = !free_look;
                                self.hud.free_look(free_look);
                            },
                            (PressBehavior::Hold, state) => {
                                free_look = state;
                                self.hud.free_look(free_look);
                            },
                            _ => {},
                        };
                    },
                    Event::InputUpdate(GameInput::AutoWalk, state) => {
                        match (global_state.settings.gameplay.auto_walk_behavior, state) {
                            (PressBehavior::Toggle, true) => {
                                auto_walk = !auto_walk;
                                self.key_state.auto_walk = auto_walk;
                                self.hud.auto_walk(auto_walk);
                            },
                            (PressBehavior::Hold, state) => {
                                auto_walk = state;
                                self.key_state.auto_walk = auto_walk;
                                self.hud.auto_walk(auto_walk);
                            },
                            _ => {},
                        }
                    },
                    Event::AnalogGameInput(input) => match input {
                        AnalogGameInput::MovementX(v) => {
                            self.key_state.analog_matrix.x = v;
                        },
                        AnalogGameInput::MovementY(v) => {
                            self.key_state.analog_matrix.y = v;
                        },
                        other => {
                            self.scene.handle_input_event(Event::AnalogGameInput(other));
                        },
                    },
                    Event::ScreenshotMessage(screenshot_message) => self.hud.new_message(Chat {
                        chat_type: ChatType::Meta,
                        message: screenshot_message,
                    }),

                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    }, // TODO: Do something if the event wasn't handled?
                }
            }

            if !free_look {
                ori = self.scene.camera().get_orientation();
                self.inputs.look_dir = Dir::from_unnormalized(cam_dir + aim_dir_offset).unwrap();
            }
            // Calculate the movement input vector of the player from the current key
            // presses and the camera direction.
            let unit_vecs = (
                Vec2::new(ori[0].cos(), -ori[0].sin()),
                Vec2::new(ori[0].sin(), ori[0].cos()),
            );
            let dir_vec = self.key_state.dir_vec();
            self.inputs.move_dir = unit_vecs.0 * dir_vec[0] + unit_vecs.1 * dir_vec[1];

            self.inputs.climb = self.key_state.climb();

            // Runs if either in a multiplayer server or the singleplayer server is unpaused
            if global_state.singleplayer.is_none()
                || !global_state.singleplayer.as_ref().unwrap().is_paused()
            {
                // Perform an in-game tick.
                match self.tick(clock.get_avg_delta(), global_state) {
                    Ok(TickAction::Continue) => {}, // Do nothing
                    Ok(TickAction::Disconnect) => return PlayStateResult::Pop, // Go to main menu
                    Err(err) => {
                        global_state.info_message =
                            Some(localized_strings.get("common.connection_lost").to_owned());
                        error!("[session] Failed to tick the scene: {:?}", err);

                        return PlayStateResult::Pop;
                    },
                }
            }

            // Maintain global state.
            global_state.maintain(clock.get_last_delta().as_secs_f32());

            // Recompute dependents just in case some input modified the camera
            self.scene
                .camera_mut()
                .compute_dependents(&*self.client.borrow().state().terrain());
            // Extract HUD events ensuring the client borrow gets dropped.
            let mut hud_events = self.hud.maintain(
                &self.client.borrow(),
                global_state,
                DebugInfo {
                    tps: clock.get_tps(),
                    ping_ms: self.client.borrow().get_ping_ms(),
                    coordinates: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<Pos>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                    velocity: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<Vel>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                    ori: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<comp::Ori>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                    num_chunks: self.scene.terrain().chunk_count() as u32,
                    num_visible_chunks: self.scene.terrain().visible_chunk_count() as u32,
                    num_figures: self.scene.figure_mgr().figure_count() as u32,
                    num_figures_visible: self.scene.figure_mgr().figure_count_visible() as u32,
                },
                &self.scene.camera(),
                clock.get_last_delta(),
                HudInfo {
                    is_aiming,
                    is_first_person: matches!(
                        self.scene.camera().get_mode(),
                        camera::CameraMode::FirstPerson
                    ),
                },
            );

            // Look for changes in the localization files
            if localization_watcher.reloaded() {
                hud_events.push(HudEvent::ChangeLanguage(localized_strings.metadata.clone()));
            }

            // Maintain the UI.
            for event in hud_events {
                match event {
                    HudEvent::SendMessage(msg) => {
                        // TODO: Handle result
                        self.client.borrow_mut().send_chat(msg);
                    },
                    HudEvent::CharacterSelection => {
                        self.client.borrow_mut().request_remove_character()
                    },
                    HudEvent::Logout => self.client.borrow_mut().request_logout(),
                    HudEvent::Quit => {
                        return PlayStateResult::Shutdown;
                    },
                    HudEvent::AdjustMousePan(sensitivity) => {
                        global_state.window.pan_sensitivity = sensitivity;
                        global_state.settings.gameplay.pan_sensitivity = sensitivity;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustMouseZoom(sensitivity) => {
                        global_state.window.zoom_sensitivity = sensitivity;
                        global_state.settings.gameplay.zoom_sensitivity = sensitivity;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleZoomInvert(zoom_inverted) => {
                        global_state.window.zoom_inversion = zoom_inverted;
                        global_state.settings.gameplay.zoom_inversion = zoom_inverted;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::Sct(sct) => {
                        global_state.settings.gameplay.sct = sct;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SctPlayerBatch(sct_player_batch) => {
                        global_state.settings.gameplay.sct_player_batch = sct_player_batch;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SctDamageBatch(sct_damage_batch) => {
                        global_state.settings.gameplay.sct_damage_batch = sct_damage_batch;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::SpeechBubbleDarkMode(sbdm) => {
                        global_state.settings.gameplay.speech_bubble_dark_mode = sbdm;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleDebug(toggle_debug) => {
                        global_state.settings.gameplay.toggle_debug = toggle_debug;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleMouseYInvert(mouse_y_inverted) => {
                        global_state.window.mouse_y_inversion = mouse_y_inverted;
                        global_state.settings.gameplay.mouse_y_inversion = mouse_y_inverted;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleSmoothPan(smooth_pan_enabled) => {
                        global_state.settings.gameplay.smooth_pan_enable = smooth_pan_enabled;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustViewDistance(view_distance) => {
                        self.client.borrow_mut().set_view_distance(view_distance);

                        global_state.settings.graphics.view_distance = view_distance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustSpriteRenderDistance(sprite_render_distance) => {
                        global_state.settings.graphics.sprite_render_distance =
                            sprite_render_distance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustFigureLoDRenderDistance(figure_lod_render_distance) => {
                        global_state.settings.graphics.figure_lod_render_distance =
                            figure_lod_render_distance;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::CrosshairTransp(crosshair_transp) => {
                        global_state.settings.gameplay.crosshair_transp = crosshair_transp;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChatTransp(chat_transp) => {
                        global_state.settings.gameplay.chat_transp = chat_transp;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::CrosshairType(crosshair_type) => {
                        global_state.settings.gameplay.crosshair_type = crosshair_type;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::Intro(intro_show) => {
                        global_state.settings.gameplay.intro_show = intro_show;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleXpBar(xp_bar) => {
                        global_state.settings.gameplay.xp_bar = xp_bar;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleBarNumbers(bar_numbers) => {
                        global_state.settings.gameplay.bar_numbers = bar_numbers;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ToggleShortcutNumbers(shortcut_numbers) => {
                        global_state.settings.gameplay.shortcut_numbers = shortcut_numbers;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::UiScale(scale_change) => {
                        global_state.settings.gameplay.ui_scale =
                            self.hud.scale_change(scale_change);
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustMusicVolume(music_volume) => {
                        global_state.audio.set_music_volume(music_volume);

                        global_state.settings.audio.music_volume = music_volume;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::AdjustSfxVolume(sfx_volume) => {
                        global_state.audio.set_sfx_volume(sfx_volume);

                        global_state.settings.audio.sfx_volume = sfx_volume;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeAudioDevice(name) => {
                        global_state.audio.set_device(name.clone());

                        global_state.settings.audio.output = AudioOutput::Device(name);
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeMaxFPS(fps) => {
                        global_state.settings.graphics.max_fps = fps;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::UseSlot(x) => self.client.borrow_mut().use_slot(x),
                    HudEvent::SwapSlots(a, b) => self.client.borrow_mut().swap_slots(a, b),
                    HudEvent::DropSlot(x) => {
                        let mut client = self.client.borrow_mut();
                        client.drop_slot(x);
                        if let comp::slot::Slot::Equip(equip_slot) = x {
                            if let comp::slot::EquipSlot::Lantern = equip_slot {
                                client.toggle_lantern();
                            }
                        }
                    },
                    HudEvent::ChangeHotbarState(state) => {
                        let client = self.client.borrow();

                        let server = &client.server_info.name;
                        // If we are changing the hotbar state this CANNOT be None.
                        let character_id = client.active_character_id.unwrap();

                        // Get or update the ServerProfile.
                        global_state
                            .profile
                            .set_hotbar_slots(server, character_id, state.slots);

                        global_state.profile.save_to_file_warn();

                        info!("Event! -> ChangedHotbarState")
                    },
                    HudEvent::Ability3(state) => self.inputs.ability3.set_state(state),
                    HudEvent::ChangeFOV(new_fov) => {
                        global_state.settings.graphics.fov = new_fov;
                        global_state.settings.save_to_file_warn();
                        self.scene.camera_mut().set_fov_deg(new_fov);
                        self.scene
                            .camera_mut()
                            .compute_dependents(&*self.client.borrow().state().terrain());
                    },
                    HudEvent::ChangeGamma(new_gamma) => {
                        global_state.settings.graphics.gamma = new_gamma;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeAaMode(new_aa_mode) => {
                        // Do this first so if it crashes the setting isn't saved :)
                        global_state
                            .window
                            .renderer_mut()
                            .set_aa_mode(new_aa_mode)
                            .unwrap();
                        global_state.settings.graphics.aa_mode = new_aa_mode;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeCloudMode(new_cloud_mode) => {
                        // Do this first so if it crashes the setting isn't saved :)
                        global_state
                            .window
                            .renderer_mut()
                            .set_cloud_mode(new_cloud_mode)
                            .unwrap();
                        global_state.settings.graphics.cloud_mode = new_cloud_mode;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeFluidMode(new_fluid_mode) => {
                        // Do this first so if it crashes the setting isn't saved :)
                        global_state
                            .window
                            .renderer_mut()
                            .set_fluid_mode(new_fluid_mode)
                            .unwrap();
                        global_state.settings.graphics.fluid_mode = new_fluid_mode;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeLanguage(new_language) => {
                        global_state.settings.language.selected_language =
                            new_language.language_identifier;
                        localized_strings = load_watched::<Localization>(
                            &i18n_asset_key(&global_state.settings.language.selected_language),
                            &mut localization_watcher,
                        )
                        .unwrap();
                        localized_strings.log_missing_entries();
                        self.hud.update_language(localized_strings.clone());
                    },
                    HudEvent::ToggleFullscreen => {
                        global_state
                            .window
                            .toggle_fullscreen(&mut global_state.settings);
                    },
                    HudEvent::AdjustWindowSize(new_size) => {
                        global_state.window.set_size(new_size.into());
                        global_state.settings.graphics.window_size = new_size;
                        global_state.settings.save_to_file_warn();
                    },
                    HudEvent::ChangeBinding(game_input) => {
                        global_state.window.set_keybinding_mode(game_input);
                    },
                    HudEvent::ChangeFreeLookBehavior(behavior) => {
                        global_state.settings.gameplay.free_look_behavior = behavior;
                    },
                    HudEvent::ChangeAutoWalkBehavior(behavior) => {
                        global_state.settings.gameplay.auto_walk_behavior = behavior;
                    },
                    HudEvent::ChangeStopAutoWalkOnInput(state) => {
                        global_state.settings.gameplay.stop_auto_walk_on_input = state;
                    },
                }
            }

            {
                let client = self.client.borrow();
                let scene_data = SceneData {
                    state: client.state(),
                    player_entity: client.entity(),
                    loaded_distance: client.loaded_distance(),
                    view_distance: client.view_distance().unwrap_or(1),
                    tick: client.get_tick(),
                    thread_pool: client.thread_pool(),
                    gamma: global_state.settings.graphics.gamma,
                    mouse_smoothing: global_state.settings.gameplay.smooth_pan_enable,
                    sprite_render_distance: global_state.settings.graphics.sprite_render_distance
                        as f32,
                    figure_lod_render_distance: global_state
                        .settings
                        .graphics
                        .figure_lod_render_distance
                        as f32,
                    is_aiming,
                };

                // Runs if either in a multiplayer server or the singleplayer server is unpaused
                if global_state.singleplayer.is_none()
                    || !global_state.singleplayer.as_ref().unwrap().is_paused()
                {
                    self.scene.maintain(
                        global_state.window.renderer_mut(),
                        &mut global_state.audio,
                        &scene_data,
                    );
                }

                let renderer = global_state.window.renderer_mut();
                // Clear the screen
                renderer.clear();
                // Render the screen using the global renderer
                self.scene.render(
                    renderer,
                    client.state(),
                    client.entity(),
                    client.get_tick(),
                    &scene_data,
                );
                // Draw the UI to the screen
                self.hud.render(renderer, self.scene.globals());
                // Finish the frame
                renderer.flush();
            }

            // Display the frame on the window.
            global_state
                .window
                .swap_buffers()
                .expect("Failed to swap window buffers!");

            // Wait for the next tick.
            clock.tick(Duration::from_millis(
                1000 / global_state.settings.graphics.max_fps as u64,
            ));

            // Clean things up after the tick.
            self.cleanup();

            current_client_state = self.client.borrow().get_client_state();
        }

        if let ClientState::Registered = current_client_state {
            return PlayStateResult::Switch(Box::new(CharSelectionState::new(
                global_state,
                self.client.clone(),
            )));
        }

        PlayStateResult::Pop
    }

    fn name(&self) -> &'static str { "Session" }
}
