precision mediump float;

uniform mat4 view;
uniform vec3 color;

varying vec3 v_normal;

void main() {
	vec3 lightdir = normalize(vec3(2.0, 2.0,-3.0));

	vec3 world_normal = mat3(view) * v_normal;
	float ndotl = dot(world_normal, -lightdir) * 0.5 + 0.5;

	gl_FragColor = vec4(color * ndotl, 0.3);
}
