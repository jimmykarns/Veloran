#![allow(dead_code)]

use super::{super::skeleton::*, Archetype};
use crate::{
    site::BlockMask,
    util::{RandomField, Sampler},
    IndexRef,
};
use common::{
    make_case_elim,
    terrain::{Block, BlockKind},
    vol::Vox,
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Deserialize, Serialize)]
pub struct Colors {
    pub foundation: (u8, u8, u8),
    pub floor: (u8, u8, u8),
    pub roof: roof_color::PureCases<(u8, u8, u8)>,
    pub wall: wall_color::PureCases<(u8, u8, u8)>,
    pub support: support_color::PureCases<(u8, u8, u8)>,
}

pub struct ColorTheme {
    roof: RoofColor,
    wall: WallColor,
    support: SupportColor,
}

make_case_elim!(
    roof_color,
    #[repr(u32)]
    #[derive(Clone, Copy)]
    pub enum RoofColor {
        Roof1 = 0,
        Roof2 = 1,
        Roof3 = 2,
        Roof4 = 3,
        Roof5 = 4,
        Roof6 = 5,
        Roof7 = 6,
    }
);

make_case_elim!(
    wall_color,
    #[repr(u32)]
    #[derive(Clone, Copy)]
    pub enum WallColor {
        Wall1 = 0,
        Wall2 = 1,
        Wall3 = 2,
        Wall4 = 3,
        Wall5 = 4,
        Wall6 = 5,
        Wall7 = 6,
        Wall8 = 7,
        Wall9 = 8,
    }
);

make_case_elim!(
    support_color,
    #[repr(u32)]
    #[derive(Clone, Copy)]
    pub enum SupportColor {
        Support1 = 0,
        Support2 = 1,
        Support3 = 2,
        Support4 = 3,
    }
);

const ROOF_COLORS: [RoofColor; roof_color::NUM_VARIANTS] = [
    RoofColor::Roof1,
    RoofColor::Roof2,
    RoofColor::Roof3,
    RoofColor::Roof4,
    RoofColor::Roof4,
    RoofColor::Roof6,
    RoofColor::Roof7,
];

const WALL_COLORS: [WallColor; wall_color::NUM_VARIANTS] = [
    WallColor::Wall1,
    WallColor::Wall2,
    WallColor::Wall3,
    WallColor::Wall4,
    WallColor::Wall5,
    WallColor::Wall6,
    WallColor::Wall7,
    WallColor::Wall8,
    WallColor::Wall9,
];

const SUPPORT_COLORS: [SupportColor; support_color::NUM_VARIANTS] = [
    SupportColor::Support1,
    SupportColor::Support2,
    SupportColor::Support3,
    SupportColor::Support4,
];

pub struct House {
    pub colors: ColorTheme,
    pub noise: RandomField,
    pub roof_ribbing: bool,
    pub roof_ribbing_diagonal: bool,
}

#[derive(Copy, Clone)]
pub enum Pillar {
    None,
    Chimney(i32),
    Tower(i32),
}

#[derive(Copy, Clone)]
pub enum RoofStyle {
    Hip,
    Gable,
    Rounded,
}

#[derive(Copy, Clone)]
pub enum StoreyFill {
    None,
    Upper,
    All,
}

impl StoreyFill {
    fn has_lower(&self) -> bool { matches!(self, StoreyFill::All) }

    fn has_upper(&self) -> bool { !matches!(self, StoreyFill::None) }
}

#[derive(Copy, Clone)]
pub struct Attr {
    pub central_supports: bool,
    pub storey_fill: StoreyFill,
    pub roof_style: RoofStyle,
    pub mansard: i32,
    pub pillar: Pillar,
    pub levels: i32,
    pub window: BlockKind,
}

impl Attr {
    pub fn generate<R: Rng>(rng: &mut R, _locus: i32) -> Self {
        Self {
            central_supports: rng.gen(),
            storey_fill: match rng.gen_range(0, 2) {
                //0 => StoreyFill::None,
                0 => StoreyFill::Upper,
                _ => StoreyFill::All,
            },
            roof_style: match rng.gen_range(0, 3) {
                0 => RoofStyle::Hip,
                1 => RoofStyle::Gable,
                _ => RoofStyle::Rounded,
            },
            mansard: rng.gen_range(-7, 4).max(0),
            pillar: match rng.gen_range(0, 4) {
                0 => Pillar::Chimney(rng.gen_range(2, 6)),
                _ => Pillar::None,
            },
            levels: rng.gen_range(1, 3),
            window: match rng.gen_range(0, 4) {
                0 => BlockKind::Window1,
                1 => BlockKind::Window2,
                2 => BlockKind::Window3,
                _ => BlockKind::Window4,
            },
        }
    }
}

impl Archetype for House {
    type Attr = Attr;

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>) {
        let len = rng.gen_range(-8, 24).clamped(0, 20);
        let locus = 6 + rng.gen_range(0, 5);
        let branches_per_side = 1 + len as usize / 20;
        let levels = rng.gen_range(1, 3);
        let skel = Skeleton {
            offset: -rng.gen_range(0, len + 7).clamped(0, len),
            ori: if rng.gen() { Ori::East } else { Ori::North },
            root: Branch {
                len,
                attr: Attr {
                    storey_fill: StoreyFill::All,
                    mansard: 0,
                    pillar: match rng.gen_range(0, 3) {
                        0 => Pillar::Chimney(rng.gen_range(2, 6)),
                        1 => Pillar::Tower(5 + rng.gen_range(1, 5)),
                        _ => Pillar::None,
                    },
                    levels,
                    ..Attr::generate(rng, locus)
                },
                locus,
                border: 4,
                children: [1, -1]
                    .iter()
                    .map(|flip| (0..branches_per_side).map(move |i| (i, *flip)))
                    .flatten()
                    .filter_map(|(i, flip)| {
                        if rng.gen() {
                            Some((
                                i as i32 * len / (branches_per_side - 1).max(1) as i32,
                                Branch {
                                    len: rng.gen_range(8, 16) * flip,
                                    attr: Attr {
                                        levels: rng.gen_range(1, 4).min(levels),
                                        ..Attr::generate(rng, locus)
                                    },
                                    locus: (6 + rng.gen_range(0, 3)).min(locus),
                                    border: 4,
                                    children: Vec::new(),
                                },
                            ))
                        } else {
                            None
                        }
                    })
                    .collect(),
            },
        };

        let this = Self {
            colors: ColorTheme {
                roof: *ROOF_COLORS.choose(rng).unwrap(),
                wall: *WALL_COLORS.choose(rng).unwrap(),
                support: *SUPPORT_COLORS.choose(rng).unwrap(),
            },
            noise: RandomField::new(rng.gen()),
            roof_ribbing: rng.gen(),
            roof_ribbing_diagonal: rng.gen(),
        };

        (this, skel)
    }

    #[allow(clippy::if_same_then_else)] // TODO: Pending review in #587
    #[allow(clippy::int_plus_one)] // TODO: Pending review in #587
    fn draw(
        &self,
        index: IndexRef,
        _pos: Vec3<i32>,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        ori: Ori,
        locus: i32,
        _len: i32,
        attr: &Self::Attr,
    ) -> BlockMask {
        let colors = &index.colors.site.settlement.building.archetype.house;
        let roof_color = *self.colors.roof.elim_case_pure(&colors.roof);
        let wall_color = *self.colors.wall.elim_case_pure(&colors.wall);
        let support_color = *self.colors.support.elim_case_pure(&colors.support);

        let profile = Vec2::new(bound_offset.x, z);

        let make_meta = |ori| {
            Rgb::new(
                match ori {
                    Ori::East => 0,
                    Ori::North => 2,
                },
                0,
                0,
            )
        };

        let make_block = |(r, g, b)| {
            let nz = self
                .noise
                .get(Vec3::new(center_offset.x, center_offset.y, z * 8));
            BlockMask::new(
                Block::new(
                    BlockKind::Normal,
                    // TODO: Clarify exactly how this affects the color.
                    Rgb::new(r, g, b)
                        .map(|e: u8| e.saturating_add((nz & 0x0F) as u8).saturating_sub(8)),
                ),
                2,
            )
        };

        let facade_layer = 3;
        let structural_layer = facade_layer + 1;
        let internal_layer = structural_layer + 1;
        let foundation_layer = internal_layer + 1;
        let floor_layer = foundation_layer + 1;

        let foundation = make_block(colors.foundation).with_priority(foundation_layer);
        let log = make_block(support_color);
        let floor = make_block(colors.floor);
        let wall = make_block(wall_color).with_priority(facade_layer);
        let roof = make_block(roof_color).with_priority(facade_layer - 1);
        let empty = BlockMask::nothing();
        let internal = BlockMask::new(Block::empty(), internal_layer);
        let end_window = BlockMask::new(
            Block::new(attr.window, make_meta(ori.flip())),
            structural_layer,
        );
        let fire = BlockMask::new(Block::new(BlockKind::Ember, Rgb::white()), foundation_layer);

        let storey_height = 6;
        let storey = ((z - 1) / storey_height).min(attr.levels - 1);
        let floor_height = storey_height * storey;
        let ceil_height = storey_height * (storey + 1);
        let lower_width = locus - 1;
        let upper_width = locus;
        let width = if profile.y >= ceil_height {
            upper_width
        } else {
            lower_width
        };
        let foundation_height = 0 - (dist - width - 1).max(0);
        let roof_top = storey_height * attr.levels + 2 + width;

        let edge_ori = if bound_offset.x.abs() > bound_offset.y.abs() {
            if center_offset.x > 0 { 6 } else { 2 }
        } else if (center_offset.y > 0) ^ (ori == Ori::East) {
            0
        } else {
            4
        };
        let edge_ori = if ori == Ori::East {
            (edge_ori + 2) % 8
        } else {
            edge_ori
        };

        if let Pillar::Chimney(chimney_height) = attr.pillar {
            let chimney_top = roof_top + chimney_height;
            // Chimney shaft
            if center_offset.map(|e| e.abs()).reduce_max() == 0
                && profile.y >= foundation_height + 1
            {
                return if profile.y == foundation_height + 1 {
                    fire
                } else {
                    internal.with_priority(foundation_layer)
                };
            }

            // Chimney
            if center_offset.map(|e| e.abs()).reduce_max() <= 1 && profile.y < chimney_top {
                // Fireplace
                if center_offset.product() == 0
                    && profile.y > foundation_height + 1
                    && profile.y <= foundation_height + 3
                {
                    return internal;
                } else {
                    return foundation;
                }
            }
        }

        if profile.y <= foundation_height && dist < width + 3 {
            // Foundations
            if attr.storey_fill.has_lower() {
                if dist == width - 1 {
                    // Floor lining
                    return log.with_priority(floor_layer);
                } else if dist < width - 1 && profile.y == foundation_height {
                    // Floor
                    return floor.with_priority(floor_layer);
                }
            }

            if dist < width && profile.y < foundation_height && profile.y >= foundation_height - 3 {
                // Basement
                return internal;
            } else {
                return foundation.with_priority(1);
            }
        }

        // Roofs and walls
        let do_roof_wall =
            |profile: Vec2<i32>, width, dist, bound_offset: Vec2<i32>, roof_top, mansard| {
                // Roof

                let (roof_profile, roof_dist) = match &attr.roof_style {
                    RoofStyle::Hip => (Vec2::new(dist, profile.y), dist),
                    RoofStyle::Gable => (profile, dist),
                    RoofStyle::Rounded => {
                        let circular_dist = (bound_offset.map(|e| e.pow(4) as f32).sum().powf(0.25)
                            - 0.5)
                            .ceil() as i32;
                        (Vec2::new(circular_dist, profile.y), circular_dist)
                    },
                };

                let roof_level = roof_top - roof_profile.x.max(mansard);

                if profile.y > roof_level {
                    return None;
                }

                // Roof
                if profile.y == roof_level && roof_dist <= width + 2 {
                    let is_ribbing = ((profile.y - ceil_height) % 3 == 0 && self.roof_ribbing)
                        || (bound_offset.x == bound_offset.y && self.roof_ribbing_diagonal);
                    if (roof_profile.x == 0 && mansard == 0) || roof_dist == width + 2 || is_ribbing
                    {
                        // Eaves
                        return Some(log);
                    } else {
                        return Some(roof);
                    }
                }

                // Wall

                if dist == width && profile.y < roof_level {
                    // Doors
                    if center_offset.x > 0
                        && center_offset.y > 0
                        && bound_offset.x > 0
                        && bound_offset.x < width
                        && profile.y < ceil_height
                        && attr.storey_fill.has_lower()
                        && storey == 0
                    {
                        return Some(
                            if (bound_offset.x == (width - 1) / 2
                                || bound_offset.x == (width - 1) / 2 + 1)
                                && profile.y <= foundation_height + 3
                            {
                                // Doors on first floor only
                                if profile.y == foundation_height + 1 {
                                    BlockMask::new(
                                        Block::new(
                                            BlockKind::Door,
                                            if bound_offset.x == (width - 1) / 2 {
                                                make_meta(ori.flip())
                                            } else {
                                                make_meta(ori.flip()) + Rgb::new(4, 0, 0)
                                            },
                                        ),
                                        structural_layer,
                                    )
                                } else {
                                    empty.with_priority(structural_layer)
                                }
                            } else {
                                wall
                            },
                        );
                    }

                    if bound_offset.x == bound_offset.y || profile.y == ceil_height {
                        // Support beams
                        return Some(log);
                    } else if !attr.storey_fill.has_lower() && profile.y < ceil_height {
                        return Some(empty);
                    } else if !attr.storey_fill.has_upper() {
                        return Some(empty);
                    } else {
                        let (frame_bounds, frame_borders) = if profile.y >= ceil_height {
                            (
                                Aabr {
                                    min: Vec2::new(-1, ceil_height + 2),
                                    max: Vec2::new(1, ceil_height + 5),
                                },
                                Vec2::new(1, 1),
                            )
                        } else {
                            (
                                Aabr {
                                    min: Vec2::new(2, floor_height + 2),
                                    max: Vec2::new(width - 2, ceil_height - 2),
                                },
                                Vec2::new(1, 0),
                            )
                        };
                        let window_bounds = Aabr {
                            min: (frame_bounds.min + frame_borders)
                                .map2(frame_bounds.center(), |a, b| a.min(b)),
                            max: (frame_bounds.max - frame_borders)
                                .map2(frame_bounds.center(), |a, b| a.max(b)),
                        };

                        // Window
                        if (frame_bounds.size() + 1).reduce_min() > 2 {
                            // Window frame is large enough for a window
                            let surface_pos = Vec2::new(bound_offset.x, profile.y);
                            if window_bounds.contains_point(surface_pos) {
                                return Some(end_window);
                            } else if frame_bounds.contains_point(surface_pos) {
                                return Some(log.with_priority(structural_layer));
                            };
                        }

                        // Wall
                        return Some(if attr.central_supports && profile.x == 0 {
                            // Support beams
                            log.with_priority(structural_layer)
                        } else {
                            wall
                        });
                    }
                }

                if dist < width {
                    // Internals
                    if profile.y == ceil_height {
                        if profile.x == 0 {
                            // Rafters
                            return Some(log);
                        } else if attr.storey_fill.has_upper() {
                            // Ceiling
                            return Some(floor);
                        }
                    } else if (!attr.storey_fill.has_lower() && profile.y < ceil_height)
                        || (!attr.storey_fill.has_upper() && profile.y >= ceil_height)
                    {
                        return Some(empty);
                    // Furniture
                    } else if dist == width - 1
                        && center_offset.sum() % 2 == 0
                        && profile.y == floor_height + 1
                        && self
                            .noise
                            .chance(Vec3::new(center_offset.x, center_offset.y, z), 0.2)
                    {
                        let mut rng = rand::thread_rng();
                        let furniture = match self.noise.get(Vec3::new(
                            center_offset.x,
                            center_offset.y,
                            z + 100,
                        )) % 11
                        {
                            0 => BlockKind::Planter,
                            1 => BlockKind::ChairSingle,
                            2 => BlockKind::ChairDouble,
                            3 => BlockKind::CoatRack,
                            4 => {
                                if rng.gen_range(0, 8) == 0 {
                                    BlockKind::Chest
                                } else {
                                    BlockKind::Crate
                                }
                            },
                            6 => BlockKind::DrawerMedium,
                            7 => BlockKind::DrawerSmall,
                            8 => BlockKind::TableSide,
                            9 => BlockKind::WardrobeSingle,
                            _ => BlockKind::Pot,
                        };

                        return Some(BlockMask::new(
                            Block::new(furniture, Rgb::new(edge_ori, 0, 0)),
                            internal_layer,
                        ));
                    } else {
                        return Some(internal);
                    }
                }

                // Wall ornaments
                if dist == width + 1
                    && center_offset.map(|e| e.abs()).reduce_min() == 0
                    && profile.y == floor_height + 3
                    && self
                        .noise
                        .chance(Vec3::new(center_offset.x, center_offset.y, z), 0.35)
                    && attr.storey_fill.has_lower()
                {
                    let ornament =
                        match self
                            .noise
                            .get(Vec3::new(center_offset.x, center_offset.y, z + 100))
                            % 4
                        {
                            0 => BlockKind::HangingSign,
                            1 | 2 | 3 => BlockKind::HangingBasket,
                            _ => BlockKind::DungeonWallDecor,
                        };

                    Some(BlockMask::new(
                        Block::new(ornament, Rgb::new((edge_ori + 4) % 8, 0, 0)),
                        internal_layer,
                    ))
                } else {
                    None
                }
            };

        let mut cblock = empty;

        if let Some(block) =
            do_roof_wall(profile, width, dist, bound_offset, roof_top, attr.mansard)
        {
            cblock = cblock.resolve_with(block);
        }

        if let Pillar::Tower(tower_height) = attr.pillar {
            let tower_top = roof_top + tower_height;
            let profile = Vec2::new(center_offset.x.abs(), profile.y);
            let dist = center_offset.map(|e| e.abs()).reduce_max();

            if let Some(block) = do_roof_wall(
                profile,
                4,
                dist,
                center_offset.map(|e| e.abs()),
                tower_top,
                attr.mansard,
            ) {
                cblock = cblock.resolve_with(block);
            }
        }

        cblock
    }
}
