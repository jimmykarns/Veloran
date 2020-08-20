use crate::{
    client::{Client, RegionSubscription},
    Server,
};
use common::{
    comp::{self, item},
    msg::ServerMsg,
    sync::{Uid, WorldSyncExt},
};
use specs::{world::WorldExt, Entity as EcsEntity};
use tracing::error;

pub fn handle_lantern(server: &mut Server, entity: EcsEntity) {
    let ecs = server.state_mut().ecs();
    if ecs
        .read_storage::<comp::LightEmitter>()
        .get(entity)
        .map_or(false, |light| light.strength > 0.0)
    {
        server
            .state_mut()
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
    } else {
        let loadout_storage = ecs.read_storage::<comp::Loadout>();
        let lantern_opt = loadout_storage
            .get(entity)
            .and_then(|loadout| loadout.lantern.as_ref())
            .and_then(|item| {
                if let comp::item::ItemKind::Lantern(l) = &item.kind {
                    Some(l)
                } else {
                    None
                }
            });
        if let Some(lantern) = lantern_opt {
            let _ = ecs
                .write_storage::<comp::LightEmitter>()
                .insert(entity, comp::LightEmitter {
                    col: lantern.color(),
                    strength: lantern.strength(),
                    flicker: 1.0,
                    animated: true,
                });
        }
    }
}

pub fn handle_mount(server: &mut Server, mounter: EcsEntity, mountee: EcsEntity) {
    let state = server.state_mut();

    if state
        .ecs()
        .read_storage::<comp::Mounting>()
        .get(mounter)
        .is_none()
    {
        let not_mounting_yet = matches!(
            state.ecs().read_storage::<comp::MountState>().get(mountee),
            Some(comp::MountState::Unmounted)
        );

        if not_mounting_yet {
            if let (Some(mounter_uid), Some(mountee_uid)) = (
                state.ecs().uid_from_entity(mounter),
                state.ecs().uid_from_entity(mountee),
            ) {
                state.write_component(mountee, comp::MountState::MountedBy(mounter_uid));
                state.write_component(mounter, comp::Mounting(mountee_uid));
            }
        }
    }
}

pub fn handle_unmount(server: &mut Server, mounter: EcsEntity) {
    let state = server.state_mut();
    let mountee_entity = state
        .ecs()
        .write_storage::<comp::Mounting>()
        .get(mounter)
        .and_then(|mountee| state.ecs().entity_from_uid(mountee.0.into()));
    if let Some(mountee_entity) = mountee_entity {
        state
            .ecs()
            .write_storage::<comp::MountState>()
            .get_mut(mountee_entity)
            .map(|ms| *ms = comp::MountState::Unmounted);
    }
    state.delete_component::<comp::Mounting>(mounter);
}

#[allow(clippy::nonminimal_bool)] // TODO: Pending review in #587
pub fn handle_possess(server: &Server, possessor_uid: Uid, possesse_uid: Uid) {
    let state = &server.state;
    let ecs = state.ecs();
    if let (Some(possessor), Some(possesse)) = (
        ecs.entity_from_uid(possessor_uid.into()),
        ecs.entity_from_uid(possesse_uid.into()),
    ) {
        // Check that entities still exist
        if !(possessor.gen().is_alive() && ecs.is_alive(possessor))
            || !(possesse.gen().is_alive() && ecs.is_alive(possesse))
        {
            error!(
                "Error possessing! either the possessor entity or possesse entity no longer exists"
            );
            return;
        }

        // You can't possess other players
        let mut clients = ecs.write_storage::<Client>();
        if clients.get_mut(possesse).is_none() {
            if let Some(mut client) = clients.remove(possessor) {
                client.notify(ServerMsg::SetPlayerEntity(possesse_uid));
                clients
                    .insert(possesse, client)
                    .err()
                    .map(|e| error!(?e, "Error inserting client component during possession"));
                // Put possess item into loadout
                let mut loadouts = ecs.write_storage::<comp::Loadout>();
                let loadout = loadouts
                    .entry(possesse)
                    .expect("Could not read loadouts component while possessing")
                    .or_insert(comp::Loadout::default());

                let item = comp::Item::new_from_asset_expect("common.items.debug.possess");
                if let item::ItemKind::Tool(tool) = &item.kind {
                    let mut abilities = tool.get_abilities();
                    let mut ability_drain = abilities.drain(..);
                    let debug_item = comp::ItemConfig {
                        item,
                        ability1: ability_drain.next(),
                        ability2: ability_drain.next(),
                        ability3: ability_drain.next(),
                        block_ability: None,
                        dodge_ability: None,
                    };
                    std::mem::swap(&mut loadout.active_item, &mut loadout.second_item);
                    loadout.active_item = Some(debug_item);
                }

                // Move player component
                {
                    let mut players = ecs.write_storage::<comp::Player>();
                    if let Some(player) = players.remove(possessor) {
                        players.insert(possesse, player).err().map(|e| {
                            error!(?e, "Error inserting player component during possession")
                        });
                    }
                }
                // Transfer region subscription
                {
                    let mut subscriptions = ecs.write_storage::<RegionSubscription>();
                    if let Some(s) = subscriptions.remove(possessor) {
                        subscriptions.insert(possesse, s).err().map(|e| {
                            error!(
                                ?e,
                                "Error inserting subscription component during possession"
                            )
                        });
                    }
                }
                // Remove will of the entity
                ecs.write_storage::<comp::Agent>().remove(possesse);
                // Reset controller of former shell
                ecs.write_storage::<comp::Controller>()
                    .get_mut(possessor)
                    .map(|c| c.reset());
                // Transfer admin powers
                {
                    let mut admins = ecs.write_storage::<comp::Admin>();
                    if let Some(admin) = admins.remove(possessor) {
                        admins.insert(possesse, admin).err().map(|e| {
                            error!(?e, "Error inserting admin component during possession")
                        });
                    }
                }
                // Transfer waypoint
                {
                    let mut waypoints = ecs.write_storage::<comp::Waypoint>();
                    if let Some(waypoint) = waypoints.remove(possessor) {
                        waypoints.insert(possesse, waypoint).err().map(|e| {
                            error!(?e, "Error inserting waypoint component during possession",)
                        });
                    }
                }
            }
        }
    }
}
