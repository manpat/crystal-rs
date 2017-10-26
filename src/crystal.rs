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

struct Vertex (Vec3, usize);
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

		// for &Vertex(v, _) in self.verts.iter() {
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

		for (i, v2) in self.base_shape.iter().enumerate() {
			self.verts.push(Vertex (v2.to_x0z() * self.radius - Vec3::new(0.0, 1.0, 0.0), i*6 + 0));
			self.verts.push(Vertex (v2.to_x0z() * self.radius + Vec3::new(0.0, 1.0, 0.0), i*6 + 1));
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

		let e0 = self.split_edge(0, 0.5);
		let e1 = self.split_edge(2, 0.4);

		let v0 = self.edges[e0].vertex;
		let v1 = self.edges[e1].vertex;

		let e2 = self.connect_vertices(v0, v1);

		let e2t = self.edge_twin(e2);
		let e3 = self.split_edge(e2t, 0.5);
		let e4 = self.split_edge(1, 0.5);

		let v2 = self.edges[e3].vertex;
		let v3 = self.edges[e4].vertex;
		self.connect_vertices(v2, v3);
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

	// Returns new outgoing edge
	fn split_edge(&mut self, edge: usize, perc: f32) -> usize {
		println!("splitting edge {}", edge);

		let next = self.edge_next(edge);
		let twin = self.edge_twin(edge);
		let twinprev = self.edges[twin].prev;
		let old_twin_vert = self.edges[twin].vertex;

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

		self.verts.push(Vertex(new_vertex_pos, new_edge));
		self.edges.extend_from_slice(&new_edges);

		// Update original edge
		self.edges[edge].next = new_edge;
		self.edges[next].prev = new_edge;

		// Update original twin edge 
		self.edges[twinprev].next = new_edge_twin;
		self.edges[twin].prev = new_edge_twin;
		self.edges[twin].vertex = new_vertex;

		// Update old twins vertex outgoing edge
		if self.verts[old_twin_vert].1 == twin {
			self.verts[old_twin_vert].1 = new_edge_twin;
		}

		new_edge
	}

	// returns new edge (new edge belongs to new face)
	fn connect_vertices(&mut self, v0: usize, v1: usize) -> usize {
		println!("connecting vertices {} -> {}", v0, v1);

		let start = self.verts[v0].1;
		let mut v0_outgoing_edges = vec![start];

		let mut it = self.edge_next(self.edge_twin(start));
		while it != start {
			v0_outgoing_edges.push(it);
			assert!(self.edges[it].vertex == v0);

			it = self.edge_next(self.edge_twin(it));
		}

		let mut connecting_edge_loop = None;

		'loop_search: for start in v0_outgoing_edges {
			let mut it = self.edge_next(start);
			while it != start {
				if self.edges[it].vertex == v1 {
					connecting_edge_loop = Some((start, self.edges[it].prev));
					break 'loop_search;
				}

				it = self.edge_next(it);
			}
		}

		assert!(connecting_edge_loop.is_some(), "vertices must already be in an edge loop before connecting");

		let (v0_outgoing_edge, v1_incoming_edge) = connecting_edge_loop.unwrap();
		let v0_incoming_edge = self.edges[v0_outgoing_edge].prev;
		let v1_outgoing_edge = self.edges[v1_incoming_edge].next;

		let new_edge		= self.edges.len();
		let new_edge_twin	= new_edge + 1;

		let new_face = self.faces.len();
		let old_face = self.edges[v1_incoming_edge].face;

		let new_edges = [
			// New edge
			HalfEdge {
				vertex: v0,
				next: v1_outgoing_edge,
				twin: new_edge_twin,
				prev: v0_incoming_edge,

				face: new_face,
			},

			// New twin edge
			HalfEdge {
				vertex: v1,
				next: v0_outgoing_edge,
				twin: new_edge,
				prev: v1_incoming_edge,

				face: old_face,
			},
		];

		self.edges.extend_from_slice(&new_edges);
		self.faces.push(Face(new_edge));

		self.edges[v0_incoming_edge].next = new_edge;
		self.edges[v1_outgoing_edge].prev = new_edge;

		self.edges[v1_incoming_edge].next = new_edge_twin;
		self.edges[v0_outgoing_edge].prev = new_edge_twin;

		// Ensure the old face is pointing to the right edge loop
		self.faces[old_face].0 = new_edge_twin;

		// Update new loop with new face
		let mut it = self.edge_next(new_edge);
		while it != new_edge {
			self.edges[it].face = new_face;
			it = self.edge_next(it);
		}

		new_edge
	}

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