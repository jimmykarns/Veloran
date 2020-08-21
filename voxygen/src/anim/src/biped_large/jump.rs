use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = f64;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let torso = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();

        let wave_slow = (anim_time as f32 * 0.8).sin();

        next.head.position = Vec3::new(
            0.0,
            skeleton_attr.head.0,
            skeleton_attr.head.1 + torso * 0.2,
        ) * 1.02;
        next.head.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.position = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1 + torso * 0.5,
        );
        next.upper_torso.orientation = Quaternion::rotation_x(-0.3);
        next.upper_torso.scale = Vec3::one();

        next.lower_torso.position = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1 + torso * 0.15,
        );
        next.lower_torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.2);
        next.lower_torso.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(wave_slow * 0.09);
        next.jaw.scale = Vec3::one();

        next.tail.position = Vec3::new(
            0.0,
            skeleton_attr.tail.0,
            skeleton_attr.tail.1 + torso * 0.0,
        );
        next.tail.orientation = Quaternion::rotation_z(0.0);
        next.tail.scale = Vec3::one();

        next.control.position = Vec3::new(0.0, 0.0, 0.0);
        next.control.orientation = Quaternion::rotation_z(0.0);
        next.control.scale = Vec3::one();

        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.main.position = Vec3::new(-5.0, -7.0, 7.0);
        next.main.orientation =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.6) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one() * 1.02;

        next.shoulder_l.position = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.5);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.5);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + torso * 0.6,
        );
        next.hand_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.8);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + torso * 0.6,
        );
        next.hand_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.8);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.position = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2 + torso * 0.2,
        ) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.4);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.position = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2 + torso * 0.2,
        ) * 1.02;
        next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.4);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.position = Vec3::new(
            -skeleton_attr.foot.0,
            -5.0 + skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 8.0;
        next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.4);
        next.foot_l.scale = Vec3::one() / 8.0;

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            5.0 + skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 8.0;
        next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.4);
        next.foot_r.scale = Vec3::one() / 8.0;

        next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 8.0;

        next
    }
}
