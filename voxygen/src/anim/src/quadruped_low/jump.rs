use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head_upper.position =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.orientation = Quaternion::rotation_z(0.4) * Quaternion::rotation_x(0.0);
        next.head_upper.scale = Vec3::one();

        next.head_lower.position =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.orientation = Quaternion::rotation_z(0.2);
        next.head_lower.scale = Vec3::one();

        next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(-0.3);
        next.jaw.scale = Vec3::one() * 0.98;

        next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1)
            * skeleton_attr.scaler
            / 11.0;
        next.chest.orientation = Quaternion::rotation_y(0.0);
        next.chest.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.tail_front.position =
            Vec3::new(0.0, skeleton_attr.tail_front.0, skeleton_attr.tail_front.1);
        next.tail_front.orientation = Quaternion::rotation_x(0.15) * Quaternion::rotation_z(-0.2);
        next.tail_front.scale = Vec3::one() * 0.98;

        next.tail_rear.position =
            Vec3::new(0.0, skeleton_attr.tail_rear.0, skeleton_attr.tail_rear.1);
        next.tail_rear.orientation = Quaternion::rotation_z(-0.4) * Quaternion::rotation_x(-0.12);
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.foot_fl.position = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fl.orientation = Quaternion::rotation_z(0.3);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.position = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fr.orientation = Quaternion::rotation_z(0.3);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.position = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_bl.orientation = Quaternion::rotation_y(0.0);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.position = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_br.orientation = Quaternion::rotation_y(0.0);
        next.foot_br.scale = Vec3::one();

        next
    }
}
