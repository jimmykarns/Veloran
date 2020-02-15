use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::Tool;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct GlidingAnimation;

impl Animation for GlidingAnimation {
    type Dependency = (Option<Tool>, Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, velocity, _orientation, _last_ori, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();

        let wave_slow = (anim_time as f32 * 7.0).sin();
        let wave_slow_cos = (anim_time as f32 * 7.0).cos();
        let wave_stop = (anim_time as f32 * 1.5).min(PI / 2.0).sin();
        let wave_very_slow = (anim_time as f32 * 3.0).sin();
        let wave_very_slow_alt = (anim_time as f32 * 2.5).sin();
        let wave_very_slow_cos = (anim_time as f32 * 3.0).cos();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            -2.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 12.0,
        );
        next.head.ori = Quaternion::rotation_x(0.35 - wave_very_slow * 0.10 + head_look.y)
            * Quaternion::rotation_z(head_look.x + wave_very_slow_cos * 0.15);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, -2.0);
        next.chest.ori = Quaternion::rotation_z(wave_very_slow_cos * 0.2);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, -4.0);
        next.belt.ori = Quaternion::rotation_z(wave_very_slow_cos * 0.25);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, -7.0);
        next.shorts.ori = Quaternion::rotation_z(wave_very_slow_cos * 0.25);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -9.5 + wave_very_slow_cos * -1.5,
            -3.0 + wave_very_slow_cos * 1.5,
            6.0,
        );
        next.l_hand.ori = Quaternion::rotation_x(-2.7 + wave_very_slow_cos * -0.1);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            9.5 + wave_very_slow_cos * -1.5,
            -3.0 + wave_very_slow_cos * -1.5,
            6.0,
        );
        next.r_hand.ori = Quaternion::rotation_x(-2.7 + wave_very_slow_cos * -0.10);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 1.0, -2.0);
        next.l_foot.ori = Quaternion::rotation_x(
            (wave_stop * -0.7 - wave_slow_cos * -0.21 + wave_very_slow * 0.19) * speed * 0.04,
        );

        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 1.0, -2.0);
        next.r_foot.ori = Quaternion::rotation_x(
            (wave_stop * -0.8 + wave_slow * -0.25 + wave_very_slow_alt * 0.13) * speed * 0.04,
        );
        next.r_foot.scale = Vec3::one();

        next.main.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, -13.0 + wave_very_slow * 0.10, 6.0);
        next.glider.ori =
            Quaternion::rotation_x(1.0) * Quaternion::rotation_y(wave_very_slow_cos * 0.04);
        next.glider.scale = Vec3::one();

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, 6.0, 15.0) / 11.0 * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(-0.05 * speed.max(12.0) + wave_very_slow * 0.10);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
