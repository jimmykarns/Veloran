pub mod alpha;
pub mod idle;
pub mod jump;
pub mod run;
pub mod wield;

// Reexports
pub use self::{
    alpha::AlphaAnimation, idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation,
    wield::WieldAnimation,
};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct BipedLargeSkeleton {
    head: Bone,
    jaw: Bone,
    upper_torso: Bone,
    lower_torso: Bone,
    tail: Bone,
    main: Bone,
    second: Bone,
    shoulder_l: Bone,
    shoulder_r: Bone,
    hand_l: Bone,
    hand_r: Bone,
    leg_l: Bone,
    leg_r: Bone,
    foot_l: Bone,
    foot_r: Bone,
    torso: Bone,
    control: Bone,
}

impl BipedLargeSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for BipedLargeSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"biped_large_compute_mats\0";

    fn bone_count(&self) -> usize { 15 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_compute_mats")]
    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let jaw_mat = self.jaw.compute_base_matrix();
        let upper_torso_mat = self.upper_torso.compute_base_matrix();
        let lower_torso_mat = self.lower_torso.compute_base_matrix();
        let tail_mat = self.tail.compute_base_matrix();
        let main_mat = self.main.compute_base_matrix();
        let second_mat = self.second.compute_base_matrix();
        let shoulder_l_mat = self.shoulder_l.compute_base_matrix();
        let shoulder_r_mat = self.shoulder_r.compute_base_matrix();
        let hand_l_mat = self.hand_l.compute_base_matrix();
        let hand_r_mat = self.hand_r.compute_base_matrix();
        let leg_l_mat = self.leg_l.compute_base_matrix();
        let leg_r_mat = self.leg_r.compute_base_matrix();
        let torso_mat = self.torso.compute_base_matrix();
        let control_mat = self.control.compute_base_matrix();

        (
            [
                FigureBoneData::new(torso_mat * upper_torso_mat * self.head.compute_base_matrix()),
                FigureBoneData::new(
                    torso_mat * upper_torso_mat * self.head.compute_base_matrix() * jaw_mat,
                ),
                FigureBoneData::new(torso_mat * upper_torso_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * lower_torso_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * lower_torso_mat * tail_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * control_mat * main_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * control_mat * second_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * shoulder_l_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * shoulder_r_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * control_mat * hand_l_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * control_mat * hand_r_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * lower_torso_mat * leg_l_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * lower_torso_mat * leg_r_mat),
                FigureBoneData::new(self.foot_l.compute_base_matrix()),
                FigureBoneData::new(self.foot_r.compute_base_matrix()),
                FigureBoneData::default(),
            ],
            Vec3::default(),
        )
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.jaw.interpolate(&target.jaw, dt);
        self.upper_torso.interpolate(&target.upper_torso, dt);
        self.lower_torso.interpolate(&target.lower_torso, dt);
        self.tail.interpolate(&target.tail, dt);
        self.main.interpolate(&target.main, dt);
        self.second.interpolate(&target.second, dt);
        self.shoulder_l.interpolate(&target.shoulder_l, dt);
        self.shoulder_r.interpolate(&target.shoulder_r, dt);
        self.hand_l.interpolate(&target.hand_l, dt);
        self.hand_r.interpolate(&target.hand_r, dt);
        self.leg_l.interpolate(&target.leg_l, dt);
        self.leg_r.interpolate(&target.leg_r, dt);
        self.foot_l.interpolate(&target.foot_l, dt);
        self.foot_r.interpolate(&target.foot_r, dt);
        self.torso.interpolate(&target.torso, dt);
        self.control.interpolate(&target.control, dt);
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    jaw: (f32, f32),
    upper_torso: (f32, f32),
    lower_torso: (f32, f32),
    tail: (f32, f32),
    shoulder: (f32, f32, f32),
    hand: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
    beast: bool,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BipedLarge(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            jaw: (0.0, 0.0),
            upper_torso: (0.0, 0.0),
            lower_torso: (0.0, 0.0),
            tail: (0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            beast: false,
        }
    }
}

impl<'a> From<&'a comp::biped_large::Body> for SkeletonAttr {
    fn from(body: &'a comp::biped_large::Body) -> Self {
        use comp::biped_large::{BodyType::*, Species::*};
        Self {
            head: match (body.species, body.body_type) {
                (Ogre, Male) => (3.0, 9.0),
                (Ogre, Female) => (1.0, 7.5),
                (Cyclops, _) => (4.5, 7.5),
                (Wendigo, _) => (3.0, 13.5),
                (Troll, _) => (6.0, 10.0),
                (Dullahan, _) => (3.0, 6.0),
                (Werewolf, _) => (19.0, 1.0),
            },
            jaw: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 0.0),
                (Cyclops, _) => (0.0, 0.0),
                (Wendigo, _) => (0.0, 0.0),
                (Troll, _) => (2.0, -4.0),
                (Dullahan, _) => (0.0, 0.0),
                (Werewolf, _) => (-2.5, -4.5),
            },
            upper_torso: match (body.species, body.body_type) {
                (Ogre, Male) => (0.0, 28.0),
                (Ogre, Female) => (0.0, 28.0),
                (Cyclops, _) => (-2.0, 27.0),
                (Wendigo, _) => (-1.0, 29.0),
                (Troll, _) => (-1.0, 27.5),
                (Dullahan, _) => (0.0, 29.0),
                (Werewolf, _) => (3.0, 26.5),
            },
            lower_torso: match (body.species, body.body_type) {
                (Ogre, Male) => (1.0, -7.0),
                (Ogre, Female) => (0.0, -6.0),
                (Cyclops, _) => (1.0, -4.5),
                (Wendigo, _) => (-1.5, -6.0),
                (Troll, _) => (1.0, -10.5),
                (Dullahan, _) => (0.0, -6.5),
                (Werewolf, _) => (1.0, -10.0),
            },
            tail: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 0.0),
                (Cyclops, _) => (0.0, 0.0),
                (Wendigo, _) => (0.0, 0.0),
                (Troll, _) => (0.0, 0.0),
                (Dullahan, _) => (0.0, 0.0),
                (Werewolf, _) => (-5.5, -2.0),
            },
            shoulder: match (body.species, body.body_type) {
                (Ogre, Male) => (12.0, 0.5, 0.0),
                (Ogre, Female) => (8.0, 0.5, -1.0),
                (Cyclops, _) => (9.5, 2.5, 2.5),
                (Wendigo, _) => (9.0, 0.5, -0.5),
                (Troll, _) => (11.0, 0.5, -1.5),
                (Dullahan, _) => (14.0, 0.5, 4.5),
                (Werewolf, _) => (9.0, 4.0, -6.5),
            },
            hand: match (body.species, body.body_type) {
                (Ogre, Male) => (14.5, 0.0, -2.0),
                (Ogre, Female) => (9.0, 0.5, -4.5),
                (Cyclops, _) => (10.0, 2.0, -0.5),
                (Wendigo, _) => (12.0, 0.0, -0.5),
                (Troll, _) => (11.5, 0.0, -1.5),
                (Dullahan, _) => (14.5, 0.0, -2.5),
                (Werewolf, _) => (10.0, 2.5, -11.0),
            },
            leg: match (body.species, body.body_type) {
                (Ogre, Male) => (0.0, 0.0, -4.0),
                (Ogre, Female) => (0.0, 0.0, -2.0),
                (Cyclops, _) => (0.0, 0.0, -5.0),
                (Wendigo, _) => (2.0, 2.0, -2.5),
                (Troll, _) => (5.0, 0.0, -6.0),
                (Dullahan, _) => (0.0, 0.0, -5.0),
                (Werewolf, _) => (4.5, 0.5, -3.0),
            },
            foot: match (body.species, body.body_type) {
                (Ogre, Male) => (4.0, 2.5, 8.0),
                (Ogre, Female) => (4.0, 0.5, 8.0),
                (Cyclops, _) => (4.0, 0.5, 5.0),
                (Wendigo, _) => (5.0, 0.5, 6.0),
                (Troll, _) => (6.0, 0.5, 4.0),
                (Dullahan, _) => (4.0, 2.5, 8.0),
                (Werewolf, _) => (5.5, 6.5, 6.0),
            },
            beast: match (body.species, body.body_type) {
                (Werewolf, _) => (true),
                _ => (false),
            },
        }
    }
}
