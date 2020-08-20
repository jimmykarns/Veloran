use authc::Uuid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, NullStorage};
use specs_idvs::IdvStorage;
use crate::character::CharacterId;

const MAX_ALIAS_LEN: usize = 32;
pub const MAX_MOUNT_RANGE_SQR: i32 = 20000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub alias: String,
    pub character_id: Option<CharacterId>,
    pub view_distance: Option<u32>,
    uuid: Uuid,
}

impl Player {
    pub fn new(
        alias: String,
        character_id: Option<CharacterId>,
        view_distance: Option<u32>,
        uuid: Uuid,
    ) -> Self {
        Self {
            alias,
            character_id,
            view_distance,
            uuid,
        }
    }

    pub fn is_valid(&self) -> bool { Self::alias_is_valid(&self.alias) }

    pub fn alias_is_valid(alias: &str) -> bool {
        alias.chars().all(|c| c.is_alphanumeric() || c == '_') && alias.len() <= MAX_ALIAS_LEN
    }

    /// Not to be confused with uid
    pub fn uuid(&self) -> Uuid { self.uuid }
}

impl Component for Player {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Respawn;
impl Component for Respawn {
    type Storage = NullStorage<Self>;
}
