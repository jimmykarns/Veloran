use crate::{client::Client, Server, SpawnPoint, StateExt};
use common::{
    assets,
    comp::{self, object, Body, Damage, DamageSource, HealthChange, HealthSource, Player, Stats},
    msg::{PlayerListUpdate, ServerMsg},
    state::BlockChange,
    sync::{Uid, WorldSyncExt},
    sys::combat::BLOCK_ANGLE,
    terrain::{Block, TerrainGrid},
    vol::{ReadVol, Vox},
    lottery::Lottery,
};
use specs::{join::Join, Entity as EcsEntity, WorldExt};
use tracing::error;
use vek::Vec3;

pub fn handle_damage(server: &Server, uid: Uid, change: HealthChange) {
    let state = &server.state;
    let ecs = state.ecs();
    if let Some(entity) = ecs.entity_from_uid(uid.into()) {
        if let Some(stats) = ecs.write_storage::<Stats>().get_mut(entity) {
            stats.health.change_by(change);
        }
    }
}

/// Handle an entity dying. If it is a player, it will send a message to all
/// other players. If the entity that killed it had stats, then give it exp for
/// the kill. Experience given is equal to the level of the entity that was
/// killed times 10.
pub fn handle_destroy(server: &mut Server, entity: EcsEntity, cause: HealthSource) {
    let state = server.state_mut();

    // Chat message
    if let Some(player) = state.ecs().read_storage::<Player>().get(entity) {
        let msg = if let HealthSource::Attack { by }
        | HealthSource::Projectile { owner: Some(by) } = cause
        {
            state.ecs().entity_from_uid(by.into()).and_then(|attacker| {
                state
                    .ecs()
                    .read_storage::<Player>()
                    .get(attacker)
                    .map(|attacker_alias| {
                        format!("{} was killed by {}", &player.alias, &attacker_alias.alias)
                    })
            })
        } else {
            None
        }
        .unwrap_or(format!("{} died", &player.alias));

        state.notify_registered_clients(comp::ChatType::Kill.server_msg(msg));
    }

    {
        // Give EXP to the killer if entity had stats
        let mut stats = state.ecs().write_storage::<Stats>();
        if let Some(entity_stats) = stats.get(entity).cloned() {
            if let HealthSource::Attack { by } | HealthSource::Projectile { owner: Some(by) } =
                cause
            {
                state.ecs().entity_from_uid(by.into()).map(|attacker| {
                    if let Some(attacker_stats) = stats.get_mut(attacker) {
                        // TODO: Discuss whether we should give EXP by Player
                        // Killing or not.
                        attacker_stats.exp.change_by(
                            (entity_stats.body_type.base_exp()
                                + entity_stats.level.level()
                                    * entity_stats.body_type.base_exp_increase())
                                as i64,
                        );
                    }
                });
            }
        }
    }

    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::Vel(Vec3::zero()))
            .err()
            .map(|e| error!(?e, ?entity, "Failed to set zero vel on dead client"));
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| error!(?e, ?entity, "Failed to insert ForceUpdate on dead client"));
        state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::Energy>()
            .get_mut(entity)
            .map(|energy| energy.set_to(energy.maximum(), comp::EnergySource::Revive));
        let _ = state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .insert(entity, comp::CharacterState::default());
    } else if state.ecs().read_storage::<comp::Agent>().contains(entity) {
        // Replace npc with loot
        let _ = state
            .ecs()
            .write_storage()
            .insert(entity, Body::Object(object::Body::Pouch));

        let mut item_drops = state.ecs().write_storage::<comp::ItemDrop>();
        let item = if let Some(item_drop) = item_drops.get(entity).cloned() {
            item_drops.remove(entity);
            item_drop.0
        } else {
            let chosen = assets::load_expect::<Lottery<String>>("common.loot_table");
            let chosen = chosen.choose();

            assets::load_expect_cloned(chosen)
        };

        let _ = state.ecs().write_storage().insert(entity, item);

        state.ecs().write_storage::<comp::Stats>().remove(entity);
        state.ecs().write_storage::<comp::Agent>().remove(entity);
        state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::Controller>()
            .remove(entity);
    } else {
        let _ = state
            .delete_entity_recorded(entity)
            .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
    }

    // TODO: Add Delete(time_left: Duration) component
    /*
    // If not a player delete the entity
    if let Err(err) = state.delete_entity_recorded(entity) {
        error!(?e, "Failed to delete destroyed entity");
    }
    */
}

pub fn handle_land_on_ground(server: &Server, entity: EcsEntity, vel: Vec3<f32>) {
    let state = &server.state;
    if vel.z <= -30.0 {
        if let Some(stats) = state.ecs().write_storage::<comp::Stats>().get_mut(entity) {
            let falldmg = (vel.z.powi(2) / 20.0 - 40.0) * 10.0;
            let mut damage = Damage {
                healthchange: -falldmg,
                source: DamageSource::Falling,
            };
            if let Some(loadout) = state.ecs().read_storage::<comp::Loadout>().get(entity) {
                damage.modify_damage(false, loadout);
            }
            stats.health.change_by(comp::HealthChange {
                amount: damage.healthchange as i32,
                cause: comp::HealthSource::World,
            });
        }
    }
}

pub fn handle_respawn(server: &Server, entity: EcsEntity) {
    let state = &server.state;

    // Only clients can respawn
    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        let respawn_point = state
            .read_component_cloned::<comp::Waypoint>(entity)
            .map(|wp| wp.get_pos())
            .unwrap_or(state.ecs().read_resource::<SpawnPoint>().0);

        state
            .ecs()
            .write_storage::<comp::Stats>()
            .get_mut(entity)
            .map(|stats| stats.revive());
        state
            .ecs()
            .write_storage::<comp::Pos>()
            .get_mut(entity)
            .map(|pos| pos.0 = respawn_point);
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| {
                error!(
                    ?e,
                    "Error inserting ForceUpdate component when respawning client"
                )
            });
    }
}

pub fn handle_explosion(server: &Server, pos: Vec3<f32>, power: f32, owner: Option<Uid>) {
    // Go through all other entities
    let hit_range = 3.0 * power;
    let ecs = &server.state.ecs();
    for (pos_b, ori_b, character_b, stats_b, loadout_b) in (
        &ecs.read_storage::<comp::Pos>(),
        &ecs.read_storage::<comp::Ori>(),
        ecs.read_storage::<comp::CharacterState>().maybe(),
        &mut ecs.write_storage::<comp::Stats>(),
        ecs.read_storage::<comp::Loadout>().maybe(),
    )
        .join()
    {
        let distance_squared = pos.distance_squared(pos_b.0);
        // Check if it is a hit
        if !stats_b.is_dead
            // Spherical wedge shaped attack field
            // RADIUS
            && distance_squared < hit_range.powi(2)
        {
            // Weapon gives base damage
            let dmg = (1.0 - distance_squared / hit_range.powi(2)) * power * 130.0;

            let mut damage = Damage {
                healthchange: -dmg,
                source: DamageSource::Explosion,
            };

            let block = character_b.map(|c_b| c_b.is_block()).unwrap_or(false)
                && ori_b.0.angle_between(pos - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0;

            if let Some(loadout) = loadout_b {
                damage.modify_damage(block, loadout);
            }

            stats_b.health.change_by(HealthChange {
                amount: damage.healthchange as i32,
                cause: HealthSource::Projectile { owner },
            });
        }
    }

    const RAYS: usize = 500;

    // Color terrain
    let mut touched_blocks = Vec::new();
    let color_range = power * 2.7;
    for _ in 0..RAYS {
        let dir = Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
        )
        .normalized();

        let _ = ecs
            .read_resource::<TerrainGrid>()
            .ray(pos, pos + dir * color_range)
            .until(|_| rand::random::<f32>() < 0.05)
            .for_each(|_: &Block, pos| touched_blocks.push(pos))
            .cast();
    }

    let terrain = ecs.read_resource::<TerrainGrid>();
    let mut block_change = ecs.write_resource::<BlockChange>();
    for block_pos in touched_blocks {
        if let Ok(block) = terrain.get(block_pos) {
            let diff2 = block_pos.map(|b| b as f32).distance_squared(pos);
            let fade = (1.0 - diff2 / color_range.powi(2)).max(0.0);
            if let Some(mut color) = block.get_color() {
                let r = color[0] as f32 + (fade * (color[0] as f32 * 0.5 - color[0] as f32));
                let g = color[1] as f32 + (fade * (color[1] as f32 * 0.3 - color[1] as f32));
                let b = color[2] as f32 + (fade * (color[2] as f32 * 0.3 - color[2] as f32));
                color[0] = r as u8;
                color[1] = g as u8;
                color[2] = b as u8;
                block_change.set(block_pos, Block::new(block.kind(), color));
            }
        }
    }

    // Destroy terrain
    for _ in 0..RAYS {
        let dir = Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.15,
        )
        .normalized();

        let terrain = ecs.read_resource::<TerrainGrid>();
        let _ = terrain
            .ray(pos, pos + dir * power)
            .until(|block| block.is_fluid() || rand::random::<f32>() < 0.05)
            .for_each(|block: &Block, pos| {
                if block.is_explodable() {
                    block_change.set(pos, Block::empty());
                }
            })
            .cast();
    }
}

pub fn handle_level_up(server: &mut Server, entity: EcsEntity, new_level: u32) {
    let uids = server.state.ecs().read_storage::<Uid>();
    let uid = uids
        .get(entity)
        .expect("Failed to fetch uid component for entity.");

    server
        .state
        .notify_registered_clients(ServerMsg::PlayerListUpdate(PlayerListUpdate::LevelChange(
            *uid, new_level,
        )));
}
