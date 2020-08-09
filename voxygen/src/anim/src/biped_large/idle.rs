use super::{super::Animation, BipedLargeSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let torso = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();

        let slower = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.5 + PI).sin();
        
        let look = Vec2::new(
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );
        let tailmove = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

        next.head.offset = Vec3::new(
            0.0,
            skeleton_attr.head.0,
            skeleton_attr.head.1 + torso * 0.2,
        ) * 1.02;
        next.head.ori = Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.offset = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1 + torso * 0.5,
        );
        next.upper_torso.ori = Quaternion::rotation_z(0.0);
        next.upper_torso.scale = Vec3::one();

        next.lower_torso.offset = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1 + torso * 0.15,
        );
        next.lower_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.lower_torso.scale = Vec3::one() * 1.02;
        
        next.jaw.offset = Vec3::new(
            0.0,
            skeleton_attr.jaw.0 - slower * 0.12,
            skeleton_attr.jaw.1 + slow * 0.2,
        );
        next.jaw.ori = Quaternion::rotation_x(slow * 0.05);
        next.jaw.scale = Vec3::one() * 1.02;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori =
            Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one();

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_z(0.0);
        next.control.scale = Vec3::one();

        next.second.offset = Vec3::new(0.0, 0.0, 0.0);
        next.second.ori =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(-5.0, -7.0, 7.0);
        next.main.ori =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.6) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one() * 1.02;

        next.shoulder_l.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.offset = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + torso * 0.6,
        );
        next.hand_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.offset = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + torso * 0.6,
        );
        next.hand_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.offset = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2 + torso * 0.2,
        ) * 1.02;
        next.leg_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.offset = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2 + torso * 0.2,
        ) * 1.02;
        next.leg_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 8.0;
        next.foot_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_l.scale = Vec3::one() / 8.0;

        next.foot_r.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 8.0;
        next.foot_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_r.scale = Vec3::one() / 8.0;

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0) / 8.0;
        next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 8.0;

        next
    }
}
