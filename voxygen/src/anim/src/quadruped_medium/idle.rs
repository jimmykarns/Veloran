use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

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

        next.head_upper.position = Vec3::new(
            0.0,
            skeleton_attr.head_upper.0,
            skeleton_attr.head_upper.1 + slower * 0.2,
        );
        next.head_upper.orientation =
            Quaternion::rotation_z(0.3 * look.x) * Quaternion::rotation_x(0.3 * look.y);
        next.head_upper.scale = Vec3::one();

        next.head_lower.position = Vec3::new(
            0.0,
            skeleton_attr.head_lower.0,
            skeleton_attr.head_lower.1 + slower * 0.1,
        );
        next.head_lower.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(
            0.0,
            skeleton_attr.jaw.0 - slower * 0.12,
            skeleton_attr.jaw.1 + slow * 0.2,
        );
        next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);
        next.jaw.scale = Vec3::one() * 1.02;

        next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.orientation =
            Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one();

        next.torso_front.position = Vec3::new(
            0.0,
            skeleton_attr.torso_front.0,
            skeleton_attr.torso_front.1 + slower * 0.3,
        ) * skeleton_attr.scaler
            / 11.0;
        next.torso_front.orientation = Quaternion::rotation_y(slow * 0.02);
        next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.torso_back.position = Vec3::new(
            0.0,
            skeleton_attr.torso_back.0,
            skeleton_attr.torso_back.1 + slower * 0.2,
        );
        next.torso_back.orientation = Quaternion::rotation_y(-slow * 0.005)
            * Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.0);
        next.torso_back.scale = Vec3::one() * 0.99;

        next.ears.position = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.orientation = Quaternion::rotation_x(0.0 + slower * 0.03);
        next.ears.scale = Vec3::one() * 1.02;

        next.leg_fl.position = Vec3::new(
            -skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1,
            skeleton_attr.leg_f.2 + slow * -0.15 + slower * -0.15,
        );
        next.leg_fl.orientation = Quaternion::rotation_y(slow * -0.02);
        next.leg_fl.scale = Vec3::one() * 1.02;

        next.leg_fr.position = Vec3::new(
            skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1,
            skeleton_attr.leg_f.2 + slow * 0.15 + slower * -0.15,
        );
        next.leg_fr.orientation = Quaternion::rotation_y(slow * -0.02);
        next.leg_fr.scale = Vec3::one() * 1.02;

        next.leg_bl.position = Vec3::new(
            -skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1,
            skeleton_attr.leg_b.2 + slower * -0.3,
        );
        next.leg_bl.orientation = Quaternion::rotation_y(slow * -0.02);
        next.leg_bl.scale = Vec3::one() * 1.02;

        next.leg_br.position = Vec3::new(
            skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1,
            skeleton_attr.leg_b.2 + slower * -0.3,
        );
        next.leg_br.orientation = Quaternion::rotation_y(slow * -0.02);
        next.leg_br.scale = Vec3::one() * 1.02;

        next.foot_fl.position = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2 + slower * -0.2,
        );
        next.foot_fl.orientation = Quaternion::rotation_x(0.0);
        next.foot_fl.scale = Vec3::one() * 0.94;

        next.foot_fr.position = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2 + slower * -0.2,
        );
        next.foot_fr.orientation = Quaternion::rotation_x(0.0);
        next.foot_fr.scale = Vec3::one() * 0.94;

        next.foot_bl.position = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2 + slower * -0.2,
        );
        next.foot_bl.orientation = Quaternion::rotation_x(0.0);
        next.foot_bl.scale = Vec3::one() * 0.94;

        next.foot_br.position = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2 + slower * -0.2,
        );
        next.foot_br.orientation = Quaternion::rotation_x(0.0);
        next.foot_br.scale = Vec3::one() * 0.94;

        next
    }
}
