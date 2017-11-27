precision highp float;

varying vec2 v_uv;

uniform sampler2D u_color;
uniform float u_aspect;
uniform float u_time;

void main() {
	float pulse = pow(1.0 - mod(u_time / 3.0, 1.0), 6.0);
	pulse = clamp(pulse, 0.0, 1.0);

	vec2 uv = v_uv * 2.0 - 1.0;
	vec2 dir = -uv * vec2(u_aspect, 1.0) * (1.0 + pulse * 20.0) * 0.0015;

	vec4 fade_color = vec4(1.0, 0.95, 0.98, 0.998);
	fade_color = mix(fade_color, vec4(vec3(1.4), 1.0),  pulse);

	gl_FragColor = texture2D(u_color, v_uv + dir);

	if(gl_FragColor.a < 0.001) {
		gl_FragColor = vec4(0.0);
		return;
	}

	gl_FragColor *= fade_color;
	gl_FragColor.rgb *= gl_FragColor.a;
}
