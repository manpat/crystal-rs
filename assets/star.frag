precision mediump float;

varying vec3 v_normal;

uniform float u_time;

void main() {
	vec3 color = vec3(v_normal.r);

	float ang_vel = cos(v_normal.r * 73.0) * 3.0;

	vec2 p = 2.0 * (gl_PointCoord - vec2(0.5));
	float dist = length(p);
	float ang = atan(p.y, p.x);
	float off = v_normal.r * 100.0 + ang_vel * u_time;

	float a = step(dist, 0.5 + cos(ang * 4.0 + off)*0.2);

	gl_FragColor = vec4(color, a);
}
