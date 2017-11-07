precision mediump float;

varying vec3 v_normal;

void main() {
	vec3 color = vec3(v_normal.r);

	vec2 p = 2.0 * (gl_PointCoord - vec2(0.5));
	float dist = length(p);
	float ang = atan(p.y, p.x);

	float a = step(dist, 0.5 + cos(ang * 4.0)*0.2);

	gl_FragColor = vec4(color, a);
}
