attribute vec3 position;

varying vec2 v_uv;

void main() {
	gl_Position = vec4(position, 1.0);
	v_uv = position.xy * 0.5 + 0.5;
}
