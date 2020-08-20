use super::SysTimer;
use crate::{
    alias_validator::AliasValidator, client::Client, login_provider::LoginProvider,
    persistence::character::CharacterLoader, ServerSettings, CLIENT_TIMEOUT,
};
use common::{
    comp::{
        Admin, AdminList, CanBuild, ChatMode, ChatType, ControlEvent, Controller, ForceUpdate, Ori,
        Player, Pos, Stats, UnresolvedChatMsg, Vel,
    },
    event::{EventBus, ServerEvent},
    msg::{
        validate_chat_msg, CharacterInfo, ChatMsgValidationError, ClientMsg, ClientState,
        PlayerInfo, PlayerListUpdate, RequestStateError, ServerMsg, MAX_BYTES_CHAT_MSG,
    },
    state::{BlockChange, Time},
    sync::Uid,
    terrain::{Block, TerrainChunkSize, TerrainGrid},
    vol::{RectVolSize, Vox},
};
use futures_executor::block_on;
use futures_timer::Delay;
use futures_util::{select, FutureExt};
use hashbrown::HashMap;
use specs::{
    Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteExpect, WriteStorage,
};
use tracing::{debug, error, info, warn};

impl Sys {
    ///We needed to move this to a async fn, if we would use a async closures
    /// the compiler generates to much recursion and fails to compile this
    #[allow(clippy::too_many_arguments)]
    async fn handle_client_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        new_chat_msgs: &mut Vec<(Option<specs::Entity>, UnresolvedChatMsg)>,
        player_list: &HashMap<Uid, PlayerInfo>,
        new_players: &mut Vec<specs::Entity>,
        entity: specs::Entity,
        client: &mut Client,
        cnt: &mut u64,
        character_loader: &ReadExpect<'_, CharacterLoader>,
        terrain: &ReadExpect<'_, TerrainGrid>,
        uids: &ReadStorage<'_, Uid>,
        can_build: &ReadStorage<'_, CanBuild>,
        force_updates: &ReadStorage<'_, ForceUpdate>,
        stats: &mut WriteStorage<'_, Stats>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        login_provider: &mut WriteExpect<'_, LoginProvider>,
        block_changes: &mut Write<'_, BlockChange>,
        admin_list: &ReadExpect<'_, AdminList>,
        admins: &mut WriteStorage<'_, Admin>,
        positions: &mut WriteStorage<'_, Pos>,
        velocities: &mut WriteStorage<'_, Vel>,
        orientations: &mut WriteStorage<'_, Ori>,
        players: &mut WriteStorage<'_, Player>,
        controllers: &mut WriteStorage<'_, Controller>,
        settings: &Read<'_, ServerSettings>,
        alias_validator: &ReadExpect<'_, AliasValidator>,
    ) -> Result<(), crate::error::Error> {
        loop {
            let msg = client.recv().await?;
            *cnt += 1;
            match msg {
                // Go back to registered state (char selection screen)
                ClientMsg::ExitIngame => match client.client_state {
                    // Use ClientMsg::Register instead.
                    ClientState::Connected => client.error_state(RequestStateError::WrongMessage),
                    ClientState::Registered => client.error_state(RequestStateError::Already),
                    ClientState::Spectator | ClientState::Character => {
                        server_emitter.emit(ServerEvent::ExitIngame { entity });
                    },
                    ClientState::Pending => {},
                },
                // Request spectator state
                ClientMsg::Spectate => match client.client_state {
                    // Become Registered first.
                    ClientState::Connected => client.error_state(RequestStateError::Impossible),
                    ClientState::Spectator => client.error_state(RequestStateError::Already),
                    ClientState::Registered | ClientState::Character => {
                        client.allow_state(ClientState::Spectator)
                    },
                    ClientState::Pending => {},
                },
                // Request registered state (login)
                ClientMsg::Register {
                    view_distance,
                    token_or_username,
                } => {
                    let (username, uuid) =
                        match login_provider.try_login(&token_or_username, &settings.whitelist) {
                            Err(err) => {
                                client.error_state(RequestStateError::RegisterDenied(err));
                                break Ok(());
                            },
                            Ok((username, uuid)) => (username, uuid),
                        };

                    let vd =
                        view_distance.map(|vd| vd.min(settings.max_view_distance.unwrap_or(vd)));
                    let player = Player::new(username.clone(), None, vd, uuid);
                    let is_admin = admin_list.contains(&username);

                    if !player.is_valid() {
                        // Invalid player
                        client.error_state(RequestStateError::Impossible);
                        break Ok(());
                    }

                    match client.client_state {
                        ClientState::Connected => {
                            // Add Player component to this client
                            let _ = players.insert(entity, player);

                            // Give the Admin component to the player if their name exists in
                            // admin list
                            if is_admin {
                                let _ = admins.insert(entity, Admin);
                            }

                            // Tell the client its request was successful.
                            client.allow_state(ClientState::Registered);

                            // Send initial player list
                            client.notify(ServerMsg::PlayerListUpdate(PlayerListUpdate::Init(
                                player_list.clone(),
                            )));

                            // Add to list to notify all clients of the new player
                            new_players.push(entity);
                        },
                        // Use RequestState instead (No need to send `player` again).
                        _ => client.error_state(RequestStateError::Impossible),
                    }
                    //client.allow_state(ClientState::Registered);

                    // Limit view distance if it's too high
                    // This comes after state registration so that the client actually hears it
                    if settings
                        .max_view_distance
                        .zip(view_distance)
                        .map(|(max, vd)| vd > max)
                        .unwrap_or(false)
                    {
                        client.notify(ServerMsg::SetViewDistance(
                            settings.max_view_distance.unwrap_or(0),
                        ));
                    };
                },
                ClientMsg::SetViewDistance(view_distance) => {
                    if let ClientState::Character { .. } = client.client_state {
                        players.get_mut(entity).map(|player| {
                            player.view_distance = Some(
                                settings
                                    .max_view_distance
                                    .map(|max| view_distance.min(max))
                                    .unwrap_or(view_distance),
                            )
                        });

                        if settings
                            .max_view_distance
                            .map(|max| view_distance > max)
                            .unwrap_or(false)
                        {
                            client.notify(ServerMsg::SetViewDistance(
                                settings.max_view_distance.unwrap_or(0),
                            ));
                        }
                    }
                },
                ClientMsg::Character(character_id) => match client.client_state {
                    // Become Registered first.
                    ClientState::Connected => client.error_state(RequestStateError::Impossible),
                    ClientState::Registered | ClientState::Spectator => {
                        // Only send login message if it wasn't already
                        // sent previously
                        if let Some(player) = players.get(entity) {
                            // Send a request to load the character's component data from the
                            // DB. Once loaded, persisted components such as stats and inventory
                            // will be inserted for the entity
                            character_loader.load_character_data(
                                entity,
                                player.uuid().to_string(),
                                character_id,
                            );

                            // Start inserting non-persisted/default components for the entity
                            // while we load the DB data
                            server_emitter.emit(ServerEvent::InitCharacterData {
                                entity,
                                character_id,
                            });

                            // Give the player a welcome message
                            if !settings.server_description.is_empty() {
                                client.notify(
                                    ChatType::CommandInfo
                                        .server_msg(settings.server_description.clone()),
                                );
                            }

                            // Only send login message if it wasn't already
                            // sent previously
                            if !client.login_msg_sent {
                                new_chat_msgs.push((None, UnresolvedChatMsg {
                                    chat_type: ChatType::Online,
                                    message: format!("[{}] is now online.", &player.alias), // TODO: Localize this
                                }));

                                client.login_msg_sent = true;
                            }
                        } else {
                            client.notify(ServerMsg::CharacterDataLoadError(String::from(
                                "Failed to fetch player entity",
                            )))
                        }
                    },
                    ClientState::Character => client.error_state(RequestStateError::Already),
                    ClientState::Pending => {},
                },
                ClientMsg::ControllerInputs(inputs) => match client.client_state {
                    ClientState::Connected | ClientState::Registered | ClientState::Spectator => {
                        client.error_state(RequestStateError::Impossible)
                    },
                    ClientState::Character => {
                        if let Some(controller) = controllers.get_mut(entity) {
                            controller.inputs.update_with_new(inputs);
                        }
                    },
                    ClientState::Pending => {},
                },
                ClientMsg::ControlEvent(event) => match client.client_state {
                    ClientState::Connected | ClientState::Registered | ClientState::Spectator => {
                        client.error_state(RequestStateError::Impossible)
                    },
                    ClientState::Character => {
                        // Skip respawn if client entity is alive
                        if let ControlEvent::Respawn = event {
                            if stats.get(entity).map_or(true, |s| !s.is_dead) {
                                continue;
                            }
                        }
                        if let Some(controller) = controllers.get_mut(entity) {
                            controller.events.push(event);
                        }
                    },
                    ClientState::Pending => {},
                },
                ClientMsg::ControlAction(event) => match client.client_state {
                    ClientState::Connected | ClientState::Registered | ClientState::Spectator => {
                        client.error_state(RequestStateError::Impossible)
                    },
                    ClientState::Character => {
                        if let Some(controller) = controllers.get_mut(entity) {
                            controller.actions.push(event);
                        }
                    },
                    ClientState::Pending => {},
                },
                ClientMsg::ChatMsg(message) => match client.client_state {
                    ClientState::Connected => client.error_state(RequestStateError::Impossible),
                    ClientState::Registered | ClientState::Spectator | ClientState::Character => {
                        match validate_chat_msg(&message) {
                            Ok(()) => {
                                if let Some(from) = uids.get(entity) {
                                    let mode = chat_modes.get(entity).cloned().unwrap_or_default();
                                    let msg = mode.new_message(*from, message);
                                    new_chat_msgs.push((Some(entity), msg));
                                } else {
                                    error!("Could not send message. Missing player uid");
                                }
                            },
                            Err(ChatMsgValidationError::TooLong) => {
                                let max = MAX_BYTES_CHAT_MSG;
                                let len = message.len();
                                warn!(?len, ?max, "Recieved a chat message that's too long")
                            },
                        }
                    },
                    ClientState::Pending => {},
                },
                ClientMsg::PlayerPhysics { pos, vel, ori } => match client.client_state {
                    ClientState::Character => {
                        if force_updates.get(entity).is_none()
                            && stats.get(entity).map_or(true, |s| !s.is_dead)
                        {
                            let _ = positions.insert(entity, pos);
                            let _ = velocities.insert(entity, vel);
                            let _ = orientations.insert(entity, ori);
                        }
                    },
                    // Only characters can send positions.
                    _ => client.error_state(RequestStateError::Impossible),
                },
                ClientMsg::BreakBlock(pos) => {
                    if can_build.get(entity).is_some() {
                        block_changes.set(pos, Block::empty());
                    }
                },
                ClientMsg::PlaceBlock(pos, block) => {
                    if can_build.get(entity).is_some() {
                        block_changes.try_set(pos, block);
                    }
                },
                ClientMsg::TerrainChunkRequest { key } => match client.client_state {
                    ClientState::Connected | ClientState::Registered => {
                        client.error_state(RequestStateError::Impossible);
                    },
                    ClientState::Spectator | ClientState::Character => {
                        let in_vd = if let (Some(view_distance), Some(pos)) = (
                            players.get(entity).and_then(|p| p.view_distance),
                            positions.get(entity),
                        ) {
                            pos.0.xy().map(|e| e as f64).distance(
                                key.map(|e| e as f64 + 0.5)
                                    * TerrainChunkSize::RECT_SIZE.map(|e| e as f64),
                            ) < (view_distance as f64 - 1.0 + 2.5 * 2.0_f64.sqrt())
                                * TerrainChunkSize::RECT_SIZE.x as f64
                        } else {
                            true
                        };
                        if in_vd {
                            match terrain.get_key(key) {
                                Some(chunk) => client.notify(ServerMsg::TerrainChunkUpdate {
                                    key,
                                    chunk: Ok(Box::new(chunk.clone())),
                                }),
                                None => server_emitter.emit(ServerEvent::ChunkRequest(entity, key)),
                            }
                        }
                    },
                    ClientState::Pending => {},
                },
                // Always possible.
                ClientMsg::Ping => client.notify(ServerMsg::Pong),
                ClientMsg::Pong => {},
                ClientMsg::Disconnect => {
                    client.notify(ServerMsg::Disconnect);
                },
                ClientMsg::Terminate => {
                    debug!(?entity, "Client send message to termitate session");
                    server_emitter.emit(ServerEvent::ClientDisconnect(entity));
                    break Ok(());
                },
                ClientMsg::RequestCharacterList => {
                    if let Some(player) = players.get(entity) {
                        character_loader.load_character_list(entity, player.uuid().to_string())
                    }
                },
                ClientMsg::CreateCharacter { alias, tool, body } => {
                    if let Err(error) = alias_validator.validate(&alias) {
                        debug!(?error, ?alias, "denied alias as it contained a banned word");
                        client.notify(ServerMsg::CharacterActionError(error.to_string()));
                    } else if let Some(player) = players.get(entity) {
                        character_loader.create_character(
                            entity,
                            player.uuid().to_string(),
                            alias,
                            tool,
                            body,
                        );
                    }
                },
                ClientMsg::DeleteCharacter(character_id) => {
                    if let Some(player) = players.get(entity) {
                        character_loader.delete_character(
                            entity,
                            player.uuid().to_string(),
                            character_id,
                        );
                    }
                },
                ClientMsg::UnlockSkill(skill) => {
                    stats
                        .get_mut(entity)
                        .map(|s| s.skill_set.unlock_skill(skill));
                },
                ClientMsg::RefundSkill(skill) => {
                    stats
                        .get_mut(entity)
                        .map(|s| s.skill_set.refund_skill(skill));
                },
                ClientMsg::UnlockSkillGroup(skill_group_type) => {
                    stats
                        .get_mut(entity)
                        .map(|s| s.skill_set.unlock_skill_group(skill_group_type));
                },
            }
        }
    }
}

/// This system will handle new messages from clients
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, CharacterLoader>,
        ReadExpect<'a, TerrainGrid>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, ForceUpdate>,
        WriteStorage<'a, Stats>,
        ReadStorage<'a, ChatMode>,
        WriteExpect<'a, LoginProvider>,
        Write<'a, BlockChange>,
        ReadExpect<'a, AdminList>,
        WriteStorage<'a, Admin>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Controller>,
        Read<'a, ServerSettings>,
        ReadExpect<'a, AliasValidator>,
    );

    #[allow(clippy::match_ref_pats)] // TODO: Pending review in #587
    #[allow(clippy::single_char_pattern)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            character_loader,
            terrain,
            mut timer,
            uids,
            can_build,
            force_updates,
            mut stats,
            chat_modes,
            mut accounts,
            mut block_changes,
            admin_list,
            mut admins,
            mut positions,
            mut velocities,
            mut orientations,
            mut players,
            mut clients,
            mut controllers,
            settings,
            alias_validator,
        ): Self::SystemData,
    ) {
        timer.start();

        let mut server_emitter = server_event_bus.emitter();

        let mut new_chat_msgs = Vec::new();

        // Player list to send new players.
        let player_list = (&uids, &players, stats.maybe(), admins.maybe())
            .join()
            .map(|(uid, player, stats, admin)| {
                (*uid, PlayerInfo {
                    is_online: true,
                    is_admin: admin.is_some(),
                    player_alias: player.alias.clone(),
                    character: stats.map(|stats| CharacterInfo {
                        name: stats.name.clone(),
                        level: stats.level.level(),
                    }),
                })
            })
            .collect::<HashMap<_, _>>();
        // List of new players to update player lists of all clients.
        let mut new_players = Vec::new();

        for (entity, client) in (&entities, &mut clients).join() {
            let mut cnt = 0;

            let network_err: Result<(), crate::error::Error> = block_on(async {
                //TIMEOUT 0.02 ms for msg handling
                select!(
                    _ = Delay::new(std::time::Duration::from_micros(20)).fuse() => Ok(()),
                    err = Self::handle_client_msg(
                    &mut server_emitter,
                    &mut new_chat_msgs,
                    &player_list,
                    &mut new_players,
                    entity,
                    client,
                    &mut cnt,

                    &character_loader,
                    &terrain,
                    &uids,
                    &can_build,
                    &force_updates,
                    &mut stats,
                    &chat_modes,
                    &mut accounts,
                    &mut block_changes,
                    &admin_list,
                    &mut admins,
                    &mut positions,
                    &mut velocities,
                    &mut orientations,
                    &mut players,
                    &mut controllers,
                    &settings,
                    &alias_validator,
                    ).fuse() => err,
                )
            });

            // Update client ping.
            if cnt > 0 {
                client.last_ping = time.0
            } else if time.0 - client.last_ping > CLIENT_TIMEOUT
            // Timeout
            {
                info!(?entity, "timeout error with client, disconnecting");
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            } else if network_err.is_err()
            // Postbox error
            {
                debug!(?entity, "postbox error with client, disconnecting");
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            } else if time.0 - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing.
                client.notify(ServerMsg::Ping);
            }
        }

        // Handle new players.
        // Tell all clients to add them to the player list.
        for entity in new_players {
            if let (Some(uid), Some(player)) = (uids.get(entity), players.get(entity)) {
                let msg = ServerMsg::PlayerListUpdate(PlayerListUpdate::Add(*uid, PlayerInfo {
                    player_alias: player.alias.clone(),
                    is_online: true,
                    is_admin: admins.get(entity).is_some(),
                    character: None, // new players will be on character select.
                }));
                for client in (&mut clients).join().filter(|c| c.is_registered()) {
                    client.notify(msg.clone())
                }
            }
        }

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            // Handle chat commands.
            if msg.message.starts_with("/") {
                if let (Some(entity), true) = (entity, msg.message.len() > 1) {
                    let argv = String::from(&msg.message[1..]);
                    server_emitter.emit(ServerEvent::ChatCmd(entity, argv));
                }
            } else {
                // Send chat message
                server_emitter.emit(ServerEvent::Chat(msg));
            }
        }

        timer.end()
    }
}
