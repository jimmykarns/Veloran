#version 450 core

#include <globals.glsl>
#include <srgb.glsl>

layout(location=0) in uint v_pos_norm;
layout(location=1) in uint v_col_light;

layout (set = 3, binding = 0,std140)
uniform u_locals {
    vec3 model_offs;
	float load_time;
};

layout(location=0) out vec3 f_pos;
layout(location=1) flat out uint f_pos_norm;
layout(location=2) flat out vec3 f_norm;
layout(location=3) out vec3 f_col;
layout(location=4) out float f_light;

void main() {
    f_pos = vec3(
    	float((v_pos_norm >>  0) & 0x00FFu),
    	float((v_pos_norm >>  8) & 0x00FFu),
    	float((v_pos_norm >> 16) & 0x1FFFu)
    ) + model_offs;
	f_pos.z *= min(1.0001 - 0.02 / pow(tick.x - load_time, 10.0), 1.0);
	f_pos.z -= 25.0 * pow(distance(focus_pos.xy, f_pos.xy) / view_distance.x, 20.0);

    f_col = vec3(
    	float((v_col_light >>  8) & 0xFFu),
    	float((v_col_light >> 16) & 0xFFu),
    	float((v_col_light >> 24) & 0xFFu)
    ) / 255.0;

    f_light = float(v_col_light & 0xFFu) / 255.0;

	f_pos_norm = v_pos_norm;

    gl_Position =
		all_mat *
		vec4(f_pos, 1);
	// gl_Position.z = -1000.0 / (gl_Position.z + 10000.0);
}
