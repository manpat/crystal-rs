attribute vec3 position;
attribute vec3 normal;

uniform mat4 proj;

varying vec3 v_normal;

void main() {
	vec4 world_pos = vec4(position, 1.0);
	gl_Position = proj * world_pos;
	gl_PointSize = 5.0;

	v_normal = normal;
}
