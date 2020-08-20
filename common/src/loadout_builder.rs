use crate::comp::{
    item::{Item, ItemKind},
    Body, CharacterAbility, ItemConfig, Loadout,
};
use std::time::Duration;

/// Builder for character Loadouts, containing weapon and armour items belonging
/// to a character, along with some helper methods for loading Items and
/// ItemConfig
///
/// ```
/// use veloren_common::LoadoutBuilder;
///
/// // Build a loadout with character starter defaults and a specific sword with default sword abilities
/// let loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(LoadoutBuilder::default_item_config_from_str(
///         "common.items.weapons.sword.zweihander_sword_0"
///     ))
///     .build();
/// ```
pub struct LoadoutBuilder(Loadout);

impl LoadoutBuilder {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self(Loadout {
            active_item: None,
            second_item: None,
            shoulder: None,
            chest: None,
            belt: None,
            hand: None,
            pants: None,
            foot: None,
            back: None,
            ring: None,
            neck: None,
            lantern: None,
            head: None,
            tabard: None,
        })
    }

    /// Set default armor items for the loadout. This may vary with game
    /// updates, but should be safe defaults for a new character.
    pub fn defaults(self) -> Self {
        self.chest(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.rugged_chest",
        )))
        .pants(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.rugged_pants",
        )))
        .foot(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.sandals_0",
        )))
        .lantern(Some(Item::new_from_asset_expect(
            "common.items.armor.starter.lantern",
        )))
    }

    /// Default animal configuration
    pub fn animal(body: Body) -> Self {
        Self(Loadout {
            active_item: Some(ItemConfig {
                item: Item::new_from_asset_expect("common.items.weapons.empty.empty"),
                ability1: Some(CharacterAbility::BasicMelee {
                    energy_cost: 10,
                    buildup_duration: Duration::from_millis(600),
                    recover_duration: Duration::from_millis(100),
                    base_healthchange: -(body.base_dmg() as i32),
                    range: body.base_range(),
                    max_angle: 20.0,
                }),
                ability2: None,
                ability3: None,
                block_ability: None,
                dodge_ability: None,
            }),
            second_item: None,
            shoulder: None,
            chest: None,
            belt: None,
            hand: None,
            pants: None,
            foot: None,
            back: None,
            ring: None,
            neck: None,
            lantern: None,
            head: None,
            tabard: None,
        })
    }

    /// Get the default [ItemConfig](../comp/struct.ItemConfig.html) for a tool
    /// (weapon). This information is required for the `active` and `second`
    /// weapon items in a loadout. If some customisation to the item's
    /// abilities or their timings is desired, you should create and provide
    /// the item config directly to the [active_item](#method.active_item)
    /// method
    pub fn default_item_config_from_item(item: Item) -> Option<ItemConfig> {
        if let ItemKind::Tool(tool) = &item.kind {
            let mut abilities = tool.get_abilities();
            let mut ability_drain = abilities.drain(..);

            return Some(ItemConfig {
                item,
                ability1: ability_drain.next(),
                ability2: ability_drain.next(),
                ability3: ability_drain.next(),
                block_ability: Some(CharacterAbility::BasicBlock),
                dodge_ability: Some(CharacterAbility::Roll),
            });
        }

        None
    }

    /// Get an item's (weapon's) default
    /// [ItemConfig](../comp/struct.ItemConfig.html)
    /// by string reference. This will first attempt to load the Item, then
    /// the default abilities for that item via the
    /// [default_item_config_from_item](#method.default_item_config_from_item)
    /// function
    pub fn default_item_config_from_str(item_ref: &str) -> Option<ItemConfig> {
        Self::default_item_config_from_item(Item::new_from_asset_expect(item_ref))
    }

    pub fn active_item(mut self, item: Option<ItemConfig>) -> Self {
        self.0.active_item = item;

        self
    }

    pub fn second_item(mut self, item: Option<ItemConfig>) -> Self {
        self.0.second_item = item;

        self
    }

    pub fn shoulder(mut self, item: Option<Item>) -> Self {
        self.0.shoulder = item;
        self
    }

    pub fn chest(mut self, item: Option<Item>) -> Self {
        self.0.chest = item;
        self
    }

    pub fn belt(mut self, item: Option<Item>) -> Self {
        self.0.belt = item;
        self
    }

    pub fn hand(mut self, item: Option<Item>) -> Self {
        self.0.hand = item;
        self
    }

    pub fn pants(mut self, item: Option<Item>) -> Self {
        self.0.pants = item;
        self
    }

    pub fn foot(mut self, item: Option<Item>) -> Self {
        self.0.foot = item;
        self
    }

    pub fn back(mut self, item: Option<Item>) -> Self {
        self.0.back = item;
        self
    }

    pub fn ring(mut self, item: Option<Item>) -> Self {
        self.0.ring = item;
        self
    }

    pub fn neck(mut self, item: Option<Item>) -> Self {
        self.0.neck = item;
        self
    }

    pub fn lantern(mut self, item: Option<Item>) -> Self {
        self.0.lantern = item;
        self
    }

    pub fn head(mut self, item: Option<Item>) -> Self {
        self.0.head = item;
        self
    }

    pub fn tabard(mut self, item: Option<Item>) -> Self {
        self.0.tabard = item;
        self
    }

    pub fn build(self) -> Loadout { self.0 }
}
