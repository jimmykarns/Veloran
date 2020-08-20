use crate::{
    column::ColumnSample,
    sim::SimChunk,
    util::{RandomField, Sampler},
    IndexRef,
};
use common::{
    assets, comp,
    generation::{ChunkSupplement, EntityInfo},
    lottery::Lottery,
    terrain::{Block, BlockKind},
    vol::{BaseVol, ReadVol, RectSizedVol, Vox, WriteVol},
};
use noise::NoiseFn;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    f32,
    ops::{Mul, Sub},
};
use vek::*;

#[derive(Deserialize, Serialize)]
pub struct Colors {
    pub bridge: (u8, u8, u8),
    pub stalagtite: (u8, u8, u8),
}

fn close(x: f32, tgt: f32, falloff: f32) -> f32 {
    (1.0 - (x - tgt).abs() / falloff).max(0.0).powf(0.5)
}

pub fn apply_scatter_to<'a>(
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
    chunk: &SimChunk,
) {
    use BlockKind::*;
    #[allow(clippy::type_complexity)]
    // TODO: Add back all sprites we had before
    let scatter: &[(_, bool, fn(&SimChunk) -> (f32, Option<(f32, f32)>))] = &[
        // (density, Option<(wavelen, threshold)>)
        (BlueFlower, false, |c| {
            (
                close(c.temp, -0.3, 0.7).min(close(c.humidity, 0.6, 0.35)) * 0.05,
                Some((48.0, 0.6)),
            )
        }),
        (PinkFlower, false, |c| {
            (
                close(c.temp, 0.15, 0.5).min(close(c.humidity, 0.6, 0.35)) * 0.05,
                Some((48.0, 0.6)),
            )
        }),
        (DeadBush, false, |c| {
            (
                close(c.temp, 0.8, 0.3).min(close(c.humidity, 0.0, 0.4)) * 0.015,
                None,
            )
        }),
        (Twigs, false, |c| {
            ((c.tree_density - 0.5).max(0.0) * 0.001, None)
        }),
        (Stones, false, |c| {
            ((c.rockiness - 0.5).max(0.0) * 0.0008, None)
        }),
        (ShortGrass, false, |c| {
            (
                close(c.temp, 0.3, 0.4).min(close(c.humidity, 0.6, 0.35)) * 0.05,
                Some((48.0, 0.4)),
            )
        }),
        (Mushroom, false, |c| {
            (
                close(c.temp, 0.3, 0.4).min(close(c.humidity, 0.6, 0.35)) * 0.04,
                None,
            )
        }),
        (MediumGrass, false, |c| {
            (
                close(c.temp, 0.0, 0.6).min(close(c.humidity, 0.6, 0.35)) * 0.05,
                Some((48.0, 0.2)),
            )
        }),
        (LongGrass, false, |c| {
            (
                close(c.temp, 0.4, 0.4).min(close(c.humidity, 0.8, 0.2)) * 0.05,
                Some((48.0, 0.1)),
            )
        }),
        /*(GrassSnow, false, |c| {
            (
                close(c.temp, -0.4, 0.4).min(close(c.rockiness, 0.0, 0.5)) * 0.1,
                Some((48.0, 0.6)),
            )
        }),*/
    ];

    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };

            let underwater = col_sample.water_level > col_sample.alt;

            let bk = scatter
                .iter()
                .enumerate()
                .find_map(|(i, (bk, is_underwater, f))| {
                    let (density, patch) = f(chunk);
                    let is_patch = patch
                        .map(|(wavelen, threshold)| {
                            index.noise.scatter_nz.get(
                                wpos2d
                                    .map(|e| e as f64 / wavelen as f64 + i as f64 * 43.0)
                                    .into_array(),
                            ) < threshold as f64
                        })
                        .unwrap_or(false);
                    if density <= 0.0
                        || is_patch
                        || !RandomField::new(i as u32)
                            .chance(Vec3::new(wpos2d.x, wpos2d.y, 0), density)
                        || underwater != *is_underwater
                    {
                        None
                    } else {
                        Some(*bk)
                    }
                });

            if let Some(bk) = bk {
                let mut z = col_sample.alt as i32 - 4;
                for _ in 0..8 {
                    if vol
                        .get(Vec3::new(offs.x, offs.y, z))
                        .map(|b| !b.is_solid())
                        .unwrap_or(true)
                    {
                        let _ = vol.set(
                            Vec3::new(offs.x, offs.y, z),
                            Block::new(bk, Rgb::broadcast(0)),
                        );
                        break;
                    }
                    z += 1;
                }
            }
        }
    }
}

pub fn apply_paths_to<'a>(
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
) {
    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };
            let surface_z = col_sample.riverless_alt.floor() as i32;

            let noisy_color = |col: Rgb<u8>, factor: u32| {
                let nz = RandomField::new(0).get(Vec3::new(wpos2d.x, wpos2d.y, surface_z));
                col.map(|e| {
                    (e as u32 + nz % (factor * 2))
                        .saturating_sub(factor)
                        .min(255) as u8
                })
            };

            if let Some((path_dist, path_nearest, path, _)) = col_sample
                .path
                .filter(|(dist, _, path, _)| *dist < path.width)
            {
                let inset = 0;

                // Try to use the column at the centre of the path for sampling to make them
                // flatter
                let col_pos = (offs - wpos2d).map(|e| e as f32) + path_nearest;
                let col00 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 0));
                let col10 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 0));
                let col01 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 1));
                let col11 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 1));
                let col_attr = |col: &ColumnSample| {
                    Vec3::new(col.riverless_alt, col.alt, col.water_dist.unwrap_or(1000.0))
                };
                let [riverless_alt, alt, water_dist] = match (col00, col10, col01, col11) {
                    (Some(col00), Some(col10), Some(col01), Some(col11)) => Lerp::lerp(
                        Lerp::lerp(col_attr(col00), col_attr(col10), path_nearest.x.fract()),
                        Lerp::lerp(col_attr(col01), col_attr(col11), path_nearest.x.fract()),
                        path_nearest.y.fract(),
                    ),
                    _ => col_attr(col_sample),
                }
                .into_array();
                let (bridge_offset, depth) = (
                    ((water_dist.max(0.0) * 0.2).min(f32::consts::PI).cos() + 1.0) * 5.0,
                    ((1.0 - ((water_dist + 2.0) * 0.3).min(0.0).cos().abs())
                        * (riverless_alt + 5.0 - alt).max(0.0)
                        * 1.75
                        + 3.0) as i32,
                );
                let surface_z = (riverless_alt + bridge_offset).floor() as i32;

                for z in inset - depth..inset {
                    let _ = vol.set(
                        Vec3::new(offs.x, offs.y, surface_z + z),
                        if bridge_offset >= 2.0 && path_dist >= 3.0 || z < inset - 1 {
                            Block::new(
                                BlockKind::Normal,
                                noisy_color(index.colors.layer.bridge.into(), 8),
                            )
                        } else {
                            let path_color = path.surface_color(
                                col_sample.sub_surface_color.map(|e| (e * 255.0) as u8),
                            );
                            Block::new(BlockKind::Normal, noisy_color(path_color, 8))
                        },
                    );
                }
                let head_space = path.head_space(path_dist);
                for z in inset..inset + head_space {
                    let pos = Vec3::new(offs.x, offs.y, surface_z + z);
                    if vol.get(pos).unwrap().kind() != BlockKind::Water {
                        let _ = vol.set(pos, Block::empty());
                    }
                }
            }
        }
    }
}

pub fn apply_caves_to<'a>(
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
) {
    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };
            let surface_z = col_sample.riverless_alt.floor() as i32;

            if let Some((cave_dist, _, cave, _)) = col_sample
                .cave
                .filter(|(dist, _, cave, _)| *dist < cave.width)
            {
                let cave_x = (cave_dist / cave.width).min(1.0);

                // Relative units
                let cave_floor = 0.0 - 0.5 * (1.0 - cave_x.powf(2.0)).max(0.0).sqrt() * cave.width;
                let cave_height = (1.0 - cave_x.powf(2.0)).max(0.0).sqrt() * cave.width;

                // Abs units
                let cave_base = (cave.alt + cave_floor) as i32;
                let cave_roof = (cave.alt + cave_height) as i32;

                for z in cave_base..cave_roof {
                    if cave_x < 0.95
                        || index.noise.cave_nz.get(
                            Vec3::new(wpos2d.x, wpos2d.y, z)
                                .map(|e| e as f64 * 0.15)
                                .into_array(),
                        ) < 0.0
                    {
                        let _ = vol.set(Vec3::new(offs.x, offs.y, z), Block::empty());
                    }
                }

                // Stalagtites
                let stalagtites = index
                    .noise
                    .cave_nz
                    .get(wpos2d.map(|e| e as f64 * 0.125).into_array())
                    .sub(0.5)
                    .max(0.0)
                    .mul(
                        (col_sample.alt - cave_roof as f32 - 5.0)
                            .mul(0.15)
                            .clamped(0.0, 1.0) as f64,
                    )
                    .mul(45.0) as i32;

                for z in cave_roof - stalagtites..cave_roof {
                    let _ = vol.set(
                        Vec3::new(offs.x, offs.y, z),
                        Block::new(BlockKind::Rock, index.colors.layer.stalagtite.into()),
                    );
                }

                let cave_depth = (col_sample.alt - cave.alt).max(0.0);
                let difficulty = cave_depth / 100.0;

                // Scatter things in caves
                if RandomField::new(index.seed).chance(wpos2d.into(), 0.002 * difficulty.powf(1.5))
                    && cave_base < surface_z as i32 - 25
                {
                    let kind = *assets::load_expect::<Lottery<BlockKind>>("common.cave_scatter")
                        .choose_seeded(RandomField::new(index.seed + 1).get(wpos2d.into()));
                    let _ = vol.set(
                        Vec3::new(offs.x, offs.y, cave_base),
                        Block::new(kind, Rgb::zero()),
                    );
                }
            }
        }
    }
}

pub fn apply_caves_supplement<'a>(
    rng: &mut impl Rng,
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &(impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
    supplement: &mut ChunkSupplement,
) {
    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };
            let surface_z = col_sample.riverless_alt.floor() as i32;

            if let Some((cave_dist, _, cave, _)) = col_sample
                .cave
                .filter(|(dist, _, cave, _)| *dist < cave.width)
            {
                let cave_x = (cave_dist / cave.width).min(1.0);

                // Relative units
                let cave_floor = 0.0 - 0.5 * (1.0 - cave_x.powf(2.0)).max(0.0).sqrt() * cave.width;

                // Abs units
                let cave_base = (cave.alt + cave_floor) as i32;

                let cave_depth = (col_sample.alt - cave.alt).max(0.0);
                let difficulty = cave_depth / 200.0;

                // Scatter things in caves
                if RandomField::new(index.seed).chance(wpos2d.into(), 0.00005 * difficulty)
                    && cave_base < surface_z as i32 - 40
                {
                    let entity = EntityInfo::at(Vec3::new(
                        wpos2d.x as f32,
                        wpos2d.y as f32,
                        cave_base as f32,
                    ))
                    .with_alignment(comp::Alignment::Enemy)
                    .with_body(match rng.gen_range(0, 6) {
                        0 => {
                            let species = match rng.gen_range(0, 2) {
                                0 => comp::quadruped_small::Species::Truffler,
                                _ => comp::quadruped_small::Species::Hyena,
                            };
                            comp::quadruped_small::Body::random_with(rng, &species).into()
                        },
                        1 => {
                            let species = match rng.gen_range(0, 3) {
                                0 => comp::quadruped_medium::Species::Tarasque,
                                1 => comp::quadruped_medium::Species::Frostfang,
                                _ => comp::quadruped_medium::Species::Bonerattler,
                            };
                            comp::quadruped_medium::Body::random_with(rng, &species).into()
                        },
                        2 => {
                            let species = match rng.gen_range(0, 3) {
                                0 => comp::quadruped_low::Species::Maneater,
                                1 => comp::quadruped_low::Species::Rocksnapper,
                                _ => comp::quadruped_low::Species::Salamander,
                            };
                            comp::quadruped_low::Body::random_with(rng, &species).into()
                        },
                        3 => {
                            let species = match rng.gen_range(0, 3) {
                                0 => comp::critter::Species::Fungome,
                                1 => comp::critter::Species::Axolotl,
                                _ => comp::critter::Species::Rat,
                            };
                            comp::critter::Body::random_with(rng, &species).into()
                        },
                        4 => {
                            #[allow(clippy::match_single_binding)]
                            let species = match rng.gen_range(0, 1) {
                                _ => comp::golem::Species::StoneGolem,
                            };
                            comp::golem::Body::random_with(rng, &species).into()
                        },
                        _ => {
                            let species = match rng.gen_range(0, 4) {
                                0 => comp::biped_large::Species::Ogre,
                                1 => comp::biped_large::Species::Cyclops,
                                2 => comp::biped_large::Species::Wendigo,
                                _ => comp::biped_large::Species::Troll,
                            };
                            comp::biped_large::Body::random_with(rng, &species).into()
                        },
                    })
                    .with_automatic_name();

                    supplement.add_entity(entity);
                }
            }
        }
    }
}
