precision highp float;

varying vec2 v_uv;

uniform sampler2D u_bgcolor;
uniform sampler2D u_color0;
uniform sampler2D u_color1;
uniform sampler2D u_depth0;
uniform sampler2D u_depth1;

uniform mat4 proj;
uniform mat4 inv_proj;

uniform float u_refractive_index;
uniform float u_time;

// float linearise_depth(float z_b) {
// 	float zNear = 0.005;
// 	float zFar = 1000.0;

// 	float z_n = 2.0 * z_b - 1.0;
// 	return 2.0 * zNear * zFar / (zFar + zNear - z_n * (zFar - zNear));
// }

// float sample_back_depth(vec2 uv) {
// 	return linearise_depth(texture2D(u_depth0, uv).r);
// }

void main() {
	vec4 bgcolor = texture2D(u_bgcolor, v_uv);

	vec4 front_color = texture2D(u_color1, v_uv);
	vec3 front_normal = normalize(front_color.rgb * 2.0 - 1.0);

	float front_depth = texture2D(u_depth1, v_uv).r;

	vec4 world_pos = inv_proj * vec4(v_uv * 2.0 - 1.0, front_depth, 1.0);
	world_pos /= world_pos.w;

	vec3 view_dir = normalize(world_pos.xyz);

	vec3 dir = refract(view_dir, front_normal, u_refractive_index);
	
	vec3 ray_pos = world_pos.xyz;

	vec2 star_sample_pos = v_uv;
	float step = 2.0;
	float subdivisions = 4.0;

	// vec3 exit_dir = vec3(0.0);
	vec3 back_normal = vec3(0.0);

	for(float i = 0.0; i < 16.0; i += 1.0) {
		vec4 screen_pos = proj * vec4(ray_pos, 1.0);
		screen_pos /= screen_pos.w;
		screen_pos.xy = screen_pos.xy * 0.5 + 0.5;

		float depth0 = texture2D(u_depth0, screen_pos.xy).r;
		bool outside_crystal = screen_pos.z > depth0 || depth0 > 0.995;

		if(step > 0.0) {
			if(outside_crystal) {
				step /= -2.0;
				subdivisions -= 1.0;
			}
		} else if(!outside_crystal) {
			step /= -2.0;
			subdivisions -= 1.0;
		}

		ray_pos += dir * step;

		if(step > 0.0 && subdivisions < 0.0) {
			vec3 back_color = texture2D(u_color0, screen_pos.xy).rgb;
			back_normal = normalize(back_color * 2.0 - 1.0);

			vec3 exit_dir = refract(dir, -back_normal, u_refractive_index);

			ray_pos += exit_dir * 2.0;

			vec4 screen_pos = proj * vec4(ray_pos, 1.0);
			screen_pos /= screen_pos.w;

			star_sample_pos = screen_pos.xy * 0.5 + 0.5;
			break;
		}
	}

	float travel_dist = length(ray_pos - world_pos.xyz);

	const vec3 lightdir = normalize(vec3(2.0, 2.0,-1.0));
	const float inv_clarity = 0.15;
	const float light_str = 0.08;
	const float light_pwr = 0.8;

	float back_ndotl = clamp(dot(back_normal, lightdir) + 0.2, 0.0, 1.0);
	vec3 back_spec = vec3(0.1, 0.2, 0.8) * inv_clarity * back_ndotl + pow(back_ndotl, light_pwr) * light_str;

	float front_ndotl = clamp(dot(front_normal, lightdir) + 0.2, 0.0, 1.0);
	vec3 front_spec = vec3(0.0, 0.2, 1.0) * inv_clarity * front_ndotl + pow(front_ndotl, light_pwr) * light_str;

	vec3 color = front_spec + (back_spec * (1.0 - travel_dist * 0.1)) + texture2D(u_bgcolor, star_sample_pos).rgb;

	// Inner glow
	color += vec3(1.0, 0.0, 0.27) * clamp((travel_dist - 2.6) * 0.1, 0.0, 0.25);

	// vec3 color = vec3(mod(star_sample_pos, 1.0), 0.0);
	// vec3 color = vec3(mod(travel_dist * 0.1, 1.0));
	// vec3 color = vec3(travel_dist * 0.3);
	// vec3 color = front_color.rgb;
	// vec3 color = exit_dir * 0.5 + 0.5;
	// vec3 color = back_normal * 0.5 + 0.5;

	// vec3 diff = world_pos.xyz - front_world_pos.xyz;

	gl_FragColor = vec4(color * front_color.a + bgcolor.rgb * (1.0 - front_color.a), 1.0);
}
