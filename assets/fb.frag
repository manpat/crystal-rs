precision mediump float;

varying vec2 v_uv;

uniform sampler2D u_color;

void main() {
	gl_FragColor = texture2D(u_color, v_uv);
}
