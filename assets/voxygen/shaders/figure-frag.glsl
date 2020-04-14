#version 450 core

#include <globals.glsl>

layout(location=0) in vec3 f_pos;
layout(location=1) in vec3 f_col;
layout(location=2) flat in vec3 f_norm;

layout (set = 3, binding = 0,std140)
uniform u_locals {
	mat4 model_mat;
	vec4 model_col;
};

struct BoneData {
	mat4 bone_mat;
};

layout (set = 3, binding = 1,std140)
uniform u_bones {
	BoneData bones[16];
};

#include <sky.glsl>
#include <light.glsl>

layout(location=0) out vec4 tgt_color;

void main() {
	vec3 light, diffuse_light, ambient_light;
	get_sun_diffuse(f_norm, time_of_day.x, light, diffuse_light, ambient_light, 1.0);
	float point_shadow = shadow_at(f_pos, f_norm);
	diffuse_light *= point_shadow;
	ambient_light *= point_shadow;
	vec3 point_light = light_at(f_pos, f_norm);
	light += point_light;
	diffuse_light += point_light;
	vec3 surf_color = illuminate(srgb_to_linear(model_col.rgb * f_col), light, diffuse_light, ambient_light);

	float fog_level = fog(f_pos.xyz, focus_pos.xyz, medium.x);
	vec4 clouds;
	vec3 fog_color = get_sky_color(normalize(f_pos - cam_pos.xyz), time_of_day.x, cam_pos.xyz, f_pos, 0.5, true, clouds);
	vec3 color = mix(mix(surf_color, fog_color, fog_level), clouds.rgb, clouds.a);

	tgt_color = vec4(color, 1.0);
}
