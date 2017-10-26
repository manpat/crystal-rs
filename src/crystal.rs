use rendering::mesh_builder;
use math::*;

use rand::{thread_rng, Rng};

pub struct Crystal {
	pub radius: f32,

	base_shape: Vec<Vec2>,

	verts: Vec<Vertex>,
	edges: Vec<HalfEdge>,
	faces: Vec<Face>,
}

struct Vertex (Vec3);
struct Face (usize);

// https://www.openmesh.org/Daily-Builds/Doc/a00016.html
// https://en.wikipedia.org/wiki/Doubly_connected_edge_list
// http://twvideo01.ubm-us.net/o1/vault/gdc2012/slides/Programming%20Track/Rhodes_Graham_Math_for_Games_Tutorial_Computational_Geometry.pdf
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HalfEdge {
	vertex: usize,
	next: usize,
	twin: usize,
	prev: usize, // optional?

	face: usize,
}

impl Crystal {
	pub fn new() -> Self {
		Crystal {
			base_shape: Vec::new(),
			verts: Vec::new(),
			edges: Vec::new(),
			faces: Vec::new(),

			radius: 1.0,
		}
	}

	pub fn build_with(&self, mb: &mut mesh_builder::MeshBuilder) {
		use mesh_builder::Vertex as MBVert;

		// for &Vertex(v) in self.verts.iter() {
		// 	mb.add_vert(MBVert::new(v));
		// }

		let vs = &self.verts;
		let es = &self.edges;

		for &Face(start) in self.faces.iter() {
			let start_edge = &es[start];

			let mut it = start_edge.next;
			let mut ring = vec![vs[start_edge.vertex].0];

			while it != start {
				let edge = &es[it];
				it = edge.next;

				ring.push(vs[edge.vertex].0);
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
			}
		}
	}

	pub fn generate(&mut self) {
		self.verts.clear();
		self.edges.clear();

		self.generate_base_shape();

		let num_sides = self.base_shape.len();

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

					face: i,
				},

				// Top edge
				HalfEdge {
					vertex: i*2 + 1,
					next: falling_edge,
					twin: top_ring_edge,
					prev: rising_edge,

					face: i,
				},

				// Falling edge
				HalfEdge {
					vertex: j*2 + 1,
					next: bottom_edge,
					twin: next_rising_edge,
					prev: top_edge,

					face: i,
				},

				// Bottom edge
				HalfEdge {
					vertex: j*2 + 0,
					next: rising_edge,
					twin: bottom_ring_edge,
					prev: falling_edge,

					face: i,
				},

				// Bottom ring edge
				HalfEdge {
					vertex: i*2 + 0,
					next: next_bottom_ring_edge,
					twin: bottom_edge,
					prev: prev_bottom_ring_edge,

					face: num_sides,
				},

				// Top ring edge
				HalfEdge {
					vertex: j*2 + 1,
					next: prev_top_ring_edge,
					twin: top_edge,
					prev: next_top_ring_edge,

					face: num_sides + 1,
				},
			]);

			self.faces.push(Face(rising_edge));
		}

		self.faces.push(Face(4)); // bottom
		self.faces.push(Face(5)); // top

		// let plane = Plane::new(Vec3::new(0.0, 1.0, 0.0), 0.5);
		// self.clip_with_plane(&plane);
		// let num_faces = self.faces.len();
		// for i in 0..1 {
		// 	self.split_face(i);
		// }

		// let num_edges = self.edges.len();
		// for i in 0..num_edges {
		// 	self.split_edge(i, 0.5);
		// }
	}

	fn generate_base_shape(&mut self) {
		self.base_shape.clear();

		let mut rng = thread_rng();
		let num_sides: u32 = rng.gen_range(3, 6);
		let max_jitter_amt = PI / num_sides as f32;

		for i in 0..num_sides {
			let jitter = rng.gen_range(-max_jitter_amt / 2.0, max_jitter_amt / 2.0);
			let a = 2.0 * PI * i as f32 / num_sides as f32 + jitter;

			self.base_shape.push(Vec2::from_angle(a));
		}
	}

	fn split_edge(&mut self, edge: usize, perc: f32) -> usize {
		let next = self.edge_next(edge);
		let twin = self.edge_twin(edge);
		let twinprev = self.edges[twin].prev;

		let origin = self.edge_origin(edge);
		let new_vertex_pos = origin + (self.edge_origin(next) - origin) * perc;

		let new_vertex = self.verts.len();
		let num_edges = self.edges.len();

		let new_edge		= num_edges;
		let new_edge_twin	= num_edges + 1;

		let new_edges = [
			// New edge
			HalfEdge {
				vertex: new_vertex,
				next,
				twin: new_edge_twin,
				prev: edge,

				face: self.edges[edge].face,
			},

			// New twin edge
			HalfEdge {
				vertex: self.edges[twin].vertex,
				next: twin,
				twin: new_edge,
				prev: twinprev,

				face: self.edges[twin].face,
			},
		];

		self.verts.push(Vertex(new_vertex_pos));
		self.edges.extend_from_slice(&new_edges);

		// Update original edge
		self.edges[edge].next = new_edge;
		self.edges[next].prev = new_edge;

		// Update original twin edge 
		self.edges[twinprev].next = new_edge_twin;
		self.edges[twin].prev = new_edge_twin;
		self.edges[twin].vertex = new_vertex;

		new_edge
	}

	// fn split_face(&mut self, face: usize) {
	// 	let e0 = self.faces[face].0;
	// 	let e0twin = self.edges[e0].twin;
	// 	let e0next = self.edges[e0].next;
	// 	let e0twinprev = self.edges[e0twin].prev;
	// 	let e0twinface = self.edges[e0twin].face;

	// 	let e1 = self.edge_next(e0next);
	// 	let e1twin = self.edges[e1].twin;
	// 	let e1next = self.edges[e1].next;
	// 	let e1twinprev = self.edges[e1twin].prev;
	// 	let e1twinface = self.edges[e1twin].face;

	// 	let v0 = (self.edge_origin(e0) + self.edge_origin(e0next)) / 2.0;
	// 	let v1 = (self.edge_origin(e1) + self.edge_origin(e1next)) / 2.0;

	// 	let num_verts = self.verts.len();
	// 	let num_edges = self.edges.len();
	// 	let num_faces = self.faces.len();

	// 	let new_rising_edge			= num_edges;
	// 	let new_rising_edge_twin	= num_edges + 1;
	// 	let new_falling_edge		= num_edges + 2;
	// 	let new_falling_edge_twin	= num_edges + 3;
	// 	let top_split_edge			= num_edges + 4;
	// 	let bottom_split_edge		= num_edges + 5;

	// 	// Create new edges
	// 	let new_edges = [
	// 		// Rising edge
	// 		HalfEdge {
	// 			vertex: num_verts,
	// 			next: e0next,
	// 			twin: new_rising_edge_twin,
	// 			prev: e0,

	// 			face: num_faces,
	// 		},

	// 		// Twin Falling edge
	// 		HalfEdge {
	// 			vertex: self.edges[e0twin].vertex,
	// 			next: e0twin,
	// 			twin: new_rising_edge,
	// 			prev: e0twinprev,

	// 			face: self.edges[e0twin].face,
	// 		},

	// 		// Falling edge
	// 		HalfEdge {
	// 			vertex: num_verts + 1,
	// 			next: e1next,
	// 			twin: new_falling_edge_twin,
	// 			prev: e1,

	// 			face: face,
	// 		},

	// 		// Twin rising edge
	// 		HalfEdge {
	// 			vertex: self.edges[e1twin].vertex,
	// 			next: e1twin,
	// 			twin: new_falling_edge,
	// 			prev: e1twinprev,

	// 			face: self.edges[e1twin].face,
	// 		},

	// 		// Split top
	// 		HalfEdge {
	// 			vertex: num_verts + 1,
	// 			next: new_rising_edge,
	// 			twin: bottom_split_edge,
	// 			prev: e1,

	// 			face: num_faces,
	// 		},

	// 		// Split bottom
	// 		HalfEdge {
	// 			vertex: num_verts,
	// 			next: new_falling_edge,
	// 			twin: top_split_edge,
	// 			prev: e0,

	// 			face: face,
	// 		},
	// 	];

	// 	self.verts.push(Vertex(v0 * 1.4));
	// 	self.verts.push(Vertex(v1 * 1.4));

	// 	self.edges.extend_from_slice(&new_edges);

	// 	// Create new face
	// 	self.faces.push(Face(new_rising_edge));

	// 	// Update rising edge
	// 	self.edges[e0].next = bottom_split_edge;
	// 	self.edges[e0next].prev = new_rising_edge;

	// 	// Update falling edge
	// 	self.edges[e1].next = top_split_edge;
	// 	self.edges[e1next].prev = new_falling_edge;

	// 	// Update twin falling edge 
	// 	self.edges[e0twin].vertex = num_verts;
	// 	self.edges[e0twinprev].next = new_rising_edge_twin;
	// 	self.edges[e0twin].prev = new_rising_edge_twin;

	// 	// Update twin rising edge 
	// 	self.edges[e1twin].vertex = num_verts + 1;
	// 	self.edges[e1twinprev].next = new_falling_edge_twin;
	// 	self.edges[e1twin].prev = new_falling_edge_twin;

	// 	// Update new loop with new face
	// 	let mut it = new_rising_edge;
	// 	loop {
	// 		self.edges[it].face = num_faces;

	// 		it = self.edge_next(it);
	// 		if it == num_edges { break }
	// 	}
	// }

	// fn clip_with_plane(&mut self, plane: &Plane) {
	// 	let mut faces_to_remove = Vec::new();
	// 	let mut faces_to_clip = Vec::new();

	// 	for (face, &Face(start)) in self.faces.iter().enumerate() {
	// 		let mut it = start;
	// 		let mut verts_passing = 0;
	// 		let mut verts_failing = 0;

	// 		loop {				
	// 			if plane.dist(self.edge_origin(it)) > 0.0 {
	// 				verts_failing += 1;
	// 			} else {
	// 				verts_passing += 1;
	// 			}

	// 			it = self.edge_next(it);
	// 			if it == start { break }
	// 		}

	// 		if verts_passing == 0 {
	// 			faces_to_remove.push(face);
	// 		} else if verts_failing != 0 {
	// 			faces_to_clip.push(face);
	// 		}
	// 	}

	// 	println!("faces to remove: {}", faces_to_remove.len());
	// 	println!("faces to clip: {}", faces_to_clip.len());
	// 	println!("faces to leave: {}", self.faces.len() - faces_to_clip.len() - faces_to_remove.len());
	// }

	fn edge_next(&self, e: usize) -> usize {
		self.edges[e].next
	}

	fn edge_twin(&self, e: usize) -> usize {
		self.edges[e].twin
	}

	fn edge_origin(&self, e: usize) -> Vec3 {
		self.verts[self.edges[e].vertex].0
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