precision mediump float;

uniform mat4 view;
uniform vec3 color;

varying vec3 v_normal;

void main() {
	vec3 world_normal = mat3(view) * v_normal;
	gl_FragColor = vec4(world_normal * 0.5 + 0.5, 1.0);
}
