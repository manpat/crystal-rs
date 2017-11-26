precision highp float;

varying vec2 v_uv;

uniform sampler2D u_color;
uniform float u_aspect;
uniform float u_time;

void main() {
	float pulse = pow(1.0 - mod(u_time / 3.0, 1.0), 15.0);
	pulse = clamp(pulse, 0.0, 1.0);

	vec2 uv = v_uv * 2.0 - 1.0;
	// vec2 dir = uv *  * (30.0 - pulse * 25.0) + u_time;
	// dir.x = cos(dir.y + sin(u_time * 5.0 + uv.y));
	// dir.y = sin(dir.x + cos(u_time * 3.0 + uv.x));
	vec2 dir = -uv * vec2(u_aspect, 1.0) * (1.0 + pulse * 2.0) * 0.003;

	vec4 fade_color = vec4(0.9, 0.7, 0.8, 0.9);
	fade_color = mix(fade_color, vec4(1.2),  pulse);

	gl_FragColor = texture2D(u_color, v_uv + dir) * fade_color;
}
