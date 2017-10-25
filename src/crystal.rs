use rendering::mesh_builder;
use math::*;

use rand::{thread_rng, Rng};

pub struct Crystal {
	pub radius: f32,

	base_shape: Vec<Vec2>,

	verts: Vec<Vertex>,
	edges: Vec<HalfEdge>,
}

struct Vertex (Vec3);

// https://www.openmesh.org/Daily-Builds/Doc/a00016.html
// https://en.wikipedia.org/wiki/Doubly_connected_edge_list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HalfEdge {
	vertex: usize,
	next: usize,
	twin: usize,
	prev: usize, // optional?
}

// impl HalfEdge {
// 	fn get_id(&self) -> u64 {
// 		(self.vertex as u64) << 32
// 			| self.next as u64 & 0xFFFF
// 	}
// }

impl Crystal {
	pub fn new() -> Self {
		Crystal {
			base_shape: Vec::new(),
			verts: Vec::new(),
			edges: Vec::new(),

			radius: 1.0,
		}
	}

	pub fn build_with(&self, mb: &mut mesh_builder::MeshBuilder) {
		use mesh_builder::Vertex as MBVert;

		let vs = &self.verts;

		// for &Vertex(v) in self.verts.iter() {
		// 	mb.add_vert(MBVert::new(v));
		// }

		let mut visited = Vec::new();
		let mut tocheck = vec![0usize];

		while tocheck.len() > 0 {
			let heidx = *tocheck.last().unwrap();
			tocheck.pop();

			if visited.contains(&heidx) { continue }

			let mut it = heidx;
			let mut ring = Vec::new();

			loop {
				let he = &self.edges[it];
				visited.push(it);

				if !visited.contains(&he.twin) {
					tocheck.push(he.twin);
				}

				ring.push(vs[he.vertex].0);
				
				it = he.next;
				if it == heidx { break }
			}

			let center = ring.iter().fold(Vec3::zero(), |a, &v| a + v) / ring.len() as f32;

			let last = ring[0];
			ring.push(last);

			for v in ring.windows(2) {
				let d0 = v[0] - center;
				let d1 = v[1] - center;

				let gap = 0.05;
				let margin = 1.0 - gap / self.radius * 2.0;
				let margin_y = 1.0 - gap;
				mb.add_vert(MBVert::new(center + d0 * Vec3::new(margin, margin_y, margin)));
				mb.add_vert(MBVert::new(center + d1 * Vec3::new(margin, margin_y, margin)));

				// mb.add_vert(MBVert::new(center));
				// mb.add_vert(MBVert::new(v[0]));
				// mb.add_vert(MBVert::new(center));
			}
		}

		// for &HalfEdge{vertex, next, twin, prev} in self.edges.iter() {
		// 	let Vertex(v0) = vs[vertex];
		// 	let Vertex(v1) = vs[self.edges[next].vertex];

		// 	let Vertex(vt0) = vs[self.edges[twin].vertex];
		// 	let Vertex(vt1) = vs[self.edges[self.edges[twin].next].vertex];

		// 	let diff = v1 - v0;
		// 	let dir = diff.normalize();

		// 	mb.add_vert(MBVert::new(v0));
		// 	mb.add_vert(MBVert::new(v0 + dir * 0.1));

		// 	mb.add_vert(MBVert::new(v0 + diff / 2.5));
		// 	mb.add_vert(MBVert::new(vt0 + (vt1 - vt0) / 2.5));
		// }
	}

	pub fn generate(&mut self) {
		self.verts.clear();
		self.edges.clear();

		self.generate_base_shape();

		let num_sides = self.base_shape.len();
		let num_verts = num_sides * 2;

		for v2 in self.base_shape.iter() {
			self.verts.push(Vertex (v2.to_x0z() * self.radius - Vec3::new(0.0, 1.0, 0.0)));
			self.verts.push(Vertex (v2.to_x0z() * self.radius + Vec3::new(0.0, 1.0, 0.0)));
		}

		for i in 0..num_sides {
			let j = (i+1) % num_sides;
			let k = (i+num_sides-1) % num_sides;

			let rising_edge				= i*6 + 0;
			let top_edge				= i*6 + 1;
			let falling_edge			= i*6 + 2;
			let bottom_edge				= i*6 + 3;
			let bottom_ring_edge		= i*6 + 4;
			let top_ring_edge			= i*6 + 5;

			let next_rising_edge		= j*6 + 0;
			let prev_falling_edge		= k*6 + 2;

			let next_bottom_ring_edge	= j*6 + 4;
			let prev_bottom_ring_edge	= k*6 + 4;

			let next_top_ring_edge		= j*6 + 5;
			let prev_top_ring_edge		= k*6 + 5;

			self.edges.extend_from_slice(&[
				// Rising edge
				HalfEdge {
					vertex: i*2 + 0,
					next: top_edge,
					twin: prev_falling_edge,
					prev: bottom_edge,
				},

				// Top edge
				HalfEdge {
					vertex: i*2 + 1,
					next: falling_edge,
					twin: top_ring_edge,
					prev: rising_edge,
				},

				// Falling edge
				HalfEdge {
					vertex: j*2 + 1,
					next: bottom_edge,
					twin: next_rising_edge,
					prev: top_edge,
				},

				// Bottom edge
				HalfEdge {
					vertex: j*2 + 0,
					next: rising_edge,
					twin: bottom_ring_edge,
					prev: falling_edge,
				},

				// Bottom ring edge
				HalfEdge {
					vertex: i*2 + 0,
					next: next_bottom_ring_edge,
					twin: bottom_edge,
					prev: prev_bottom_ring_edge,
				},

				// Top ring edge
				HalfEdge {
					vertex: j*2 + 1,
					next: prev_top_ring_edge,
					twin: top_edge,
					prev: next_top_ring_edge,
				},
			]);
		}
	}

	fn generate_base_shape(&mut self) {
		self.base_shape.clear();

		let mut rng = thread_rng();
		let num_sides: u32 = rng.gen_range(3, 8);
		let max_jitter_amt = PI / num_sides as f32;

		for i in 0..num_sides {
			let jitter = rng.gen_range(-max_jitter_amt / 2.0, max_jitter_amt / 2.0);
			let a = 2.0 * PI * i as f32 / num_sides as f32 + jitter;

			self.base_shape.push(Vec2::from_angle(a));
		}
	}
}

struct Plane {
	normal: Vec3,
	length: f32,
}

impl Plane {
	fn new(n: Vec3, length: f32) -> Self {
		Plane {normal: n.normalize(), length}
	}

	fn dist(&self, p: Vec3) -> f32 {
		self.normal.dot(p) - self.length
	}
}