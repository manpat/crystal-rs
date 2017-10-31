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

#[derive(Copy, Clone, Debug)]
struct Vertex (Vec3, usize);

#[derive(Copy, Clone, Debug)]
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

	pub fn build_points_with(&self, mb: &mut mesh_builder::MeshBuilder) {
		use mesh_builder::Vertex as MBVert;

		for &Vertex(v, _) in self.verts.iter() {
			mb.add_vert(MBVert::new(v));
		}
	}

	pub fn build_with(&self, mb: &mut mesh_builder::MeshBuilder) {
		use mesh_builder::Vertex as MBVert;

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

		self.assert_invariants();

		self.clip_with_plane(&Plane::new(Vec3::new(0.8, 1.0, 0.0), 0.4));
		// self.clip_with_plane(&Plane::new(Vec3::new(0.0, 0.5, 0.8), 0.4));
		// self.clip_with_plane(&Plane::new(Vec3::new(0.6,-0.1,-0.8), 0.2));
		// self.clip_with_plane(&Plane::new(Vec3::new(0.6,-0.2,-0.8), 0.0));
		// self.clip_with_plane(&Plane::new(Vec3::new(0.6,-0.2,-0.8), 0.0));
		// self.clip_with_plane(&Plane::new(Vec3::new(0.6,-0.2,-0.8), 0.0));

		self.assert_invariants();
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

	fn clip_with_plane(&mut self, plane: &Plane) {
		// Contains the distances of each vertex to the plane
		let vertex_clip_data = self.verts.iter()
			.map(|&Vertex(p, _)| plane.dist(p))
			.collect::<Vec<_>>();

		#[derive(Copy, Clone, Debug)]
		enum EdgeData {
			Unseen,
			NoIntersection,
			Intersects(f32),
			TwinIntersects,
		};

		let mut edge_data = vec![EdgeData::Unseen; self.edges.len()];

		for (it, &edge) in self.edges.iter().enumerate() {
			if !match_enum!(edge_data[it], EdgeData::Unseen) { continue }

			let clip_origin = vertex_clip_data[edge.vertex];
			let clip_dest = vertex_clip_data[self.edges[edge.next].vertex];

			// Doesn't intersect plane
			if clip_origin.is_sign_positive() == clip_dest.is_sign_positive() {
				edge_data[it] = EdgeData::NoIntersection;
				edge_data[edge.twin] = EdgeData::NoIntersection;

				continue
			}

			let diff = clip_dest.abs() + clip_origin.abs();
			let intersection_point = clip_origin.abs() / diff;

			edge_data[it] = EdgeData::Intersects(intersection_point);
			edge_data[edge.twin] = EdgeData::TwinIntersects;
		}

		// Contains outgoing edges from new vertices
		let mut new_edges = Vec::new();

		// Split edges intersecting with plane
		for (it, &data) in edge_data.iter().enumerate() {
			if let EdgeData::Intersects(pt) = data {
				let new_edge = self.split_edge(it, pt);
				new_edges.push(new_edge);
				new_edges.push(self.edge_next(self.edge_twin(new_edge)));
			}
		}

		assert!(new_edges.len() != 0);

		let mut seen_faces = Vec::new();
		let mut new_face_edges = Vec::new();

		// Connect new vertices
		for &edge in new_edges.iter() {
			let face = self.edges[edge].face;

			// Faces can only be split once per plane clip,
			// 	so we only need to process each face once
			if seen_faces.contains(&face) { continue }
			seen_faces.push(face);

			let vert = self.edges[edge].vertex;
			
			// Search for the other new edge on this face
			// 	and connect em
			let mut it = self.edge_next(edge);
			while it != edge {
				if new_edges.contains(&it) {
					let vert2 = self.edges[it].vertex;
					new_face_edges.push(self.connect_vertices(vert, vert2));
					break;
				}

				it = self.edge_next(it);
			}
		}

		self.assert_invariants();

		assert!(new_face_edges.len() != 0);

		let mut faces_to_delete = Vec::new();
		for (face, &Face(start)) in self.faces.iter().enumerate() {
			let mut edge = start;
			loop {
				let v = self.edges[edge].vertex;
				if v < vertex_clip_data.len() && vertex_clip_data[v] > 0.0 {
					faces_to_delete.push(face);
					break
				}

				edge = self.edge_next(edge);
				if edge == start { break }
			}
		}

		let new_face = self.faces.len();

		for it in new_face_edges.iter_mut() {
			if faces_to_delete.contains(&self.edges[*it].face) {
				self.edges[*it].face = new_face;
			} else {
				*it = self.edge_twin(*it);

				let face = &mut self.edges[*it].face;
				assert!(faces_to_delete.contains(face));
				*face = new_face;
			}
		}

		for &edge in new_face_edges.iter() {
			let mut it = self.edge_next(edge);
			let end = self.edge_twin(edge);

			while it != end {
				if new_face_edges.contains(&it) {
					self.edges[edge].next = it;
					self.edges[it].prev = edge;
					break
				}

				it = self.edge_next(self.edge_twin(it));
			}

			assert!(it != end);
		}

		self.faces.push(Face(new_face_edges[0]));

		// TODO: It's super heavy from here on out
		// 	make it less heavy

		faces_to_delete.sort();
		let inverse_face_map = (0..self.faces.len())
			.filter(|x| faces_to_delete.binary_search(x).is_err())
			.collect::<Vec<_>>();

		let face_map = (0..self.faces.len())
			.map(|i| inverse_face_map.binary_search(&i).ok())
			.collect::<Vec<_>>();

		let inverse_vert_map = (0..self.verts.len())
			.filter(|&x| x >= vertex_clip_data.len() || vertex_clip_data[x] <= 0.0)
			.collect::<Vec<_>>();

		let vertex_map = (0..self.verts.len())
			.map(|i| inverse_vert_map.binary_search(&i).ok())
			.collect::<Vec<_>>();

		let mut edges_to_delete = Vec::new();

		for (i, edge) in self.edges.iter_mut().enumerate() {
			if let Some(face) = face_map[edge.face] {
				edge.face = face;
			} else {
				edge.face = !0;
				edges_to_delete.push(i);
				continue
			}

			if let Some(vertex) = vertex_map[edge.vertex] {
				edge.vertex = vertex;
			} else {
				edge.vertex = !0;
				edges_to_delete.push(i);
				continue
			}
		}

		self.faces = inverse_face_map.iter()
			.map(|&i| self.faces[i])
			.collect::<Vec<_>>();

		self.verts = inverse_vert_map.iter()
			.map(|&i| self.verts[i])
			.collect::<Vec<_>>();


		let inverse_edge_map = (0..self.edges.len())
			.filter(|x| edges_to_delete.binary_search(x).is_err())
			.collect::<Vec<_>>();

		let edge_map = (0..self.edges.len())
			.map(|i| inverse_edge_map.binary_search(&i).ok())
			.collect::<Vec<_>>();

		self.edges = inverse_edge_map.iter()
			.map(|&i| self.edges[i])
			.collect::<Vec<_>>();

		println!("{:?}", self.verts);
		println!("{:?}", inverse_edge_map);
		println!("{:?}", edge_map);
		println!("{:?}", self.edges);

		for edge in self.edges.iter_mut() {
			edge.next = edge_map[edge.next].unwrap_or(!0);
			edge.twin = edge_map[edge.twin].unwrap_or(!0);
			edge.prev = edge_map[edge.prev].unwrap_or(!0);
		}

		for &mut Face(ref mut edge) in self.faces.iter_mut() {
			*edge = edge_map[*edge].unwrap_or(!0);
		}

		for &mut Vertex(_, ref mut edge) in self.verts.iter_mut() {
			*edge = edge_map[*edge].unwrap_or(!0);
		}
	}

	fn assert_invariants(&self) {
		for (edge_it, edge) in self.edges.iter().enumerate() {
			assert!(edge.next < self.edges.len(), "Edge {} has invalid next", edge_it);
			assert!(edge.twin < self.edges.len(), "Edge {} has invalid twin", edge_it);
			assert!(edge.prev < self.edges.len(), "Edge {} has invalid prev", edge_it);
			assert!(edge.face < self.faces.len(), "Edge {} has invalid face", edge_it);
			assert!(edge.vertex < self.verts.len(), "Edge {} has invalid vertex", edge_it);

			assert!(edge.next != edge_it, "Edge {}'s next points to itself", edge_it);
			assert!(edge.twin != edge_it, "Edge {}'s twin points to itself", edge_it);
			assert!(edge.prev != edge_it, "Edge {}'s prev points to itself", edge_it);

			let next = &self.edges[edge.next];
			let twin = &self.edges[edge.twin];
			let prev = &self.edges[edge.prev];

			assert!(next.vertex == twin.vertex, "Edge {} next and twin have different origin vertices", edge_it);
			assert!(next.prev == edge_it, "Edge {} next edge points to an incorrect prev", edge_it);
			assert!(prev.next == edge_it, "Edge {} prev edge points to an incorrect next", edge_it);
			assert!(twin.twin == edge_it, "Edge {} twin edge points to an incorrect twin", edge_it);
		}

		for (face, &Face(mut it)) in self.faces.iter().enumerate() {
			let start = it;
			let mut loop_count = 0;

			loop {
				let edge = &self.edges[it];

				assert!(edge.face == face, "Inconsistent face pointers in edge loop");

				it = self.edge_next(it);
				if it == start { break }

				loop_count += 1;
				assert!(loop_count < 1000, "Found non-cyclic edge loop");
			}
		}

		println!("{:?}", self.verts);

		for (vertex, &Vertex(_, mut it)) in self.verts.iter().enumerate() {
			let start = it;
			let mut loop_count = 0;

			loop {
				assert!(it < self.edges.len(), "Outgoing edge of vertex {} is invalid", vertex);
				let edge = &self.edges[it];

				assert!(edge.twin < self.edges.len(), "Twin of outgoing edge of vertex {} is invalid", vertex);
				let twin = &self.edges[edge.twin];

				assert!(edge.vertex == vertex, "Edge adjacent to outgoing edge of vertex {} does not originate from same vertex", vertex);

				it = twin.next;
				if it == start { break }

				loop_count += 1;
				assert!(loop_count < 1000, "Edges adjacent to outgoing edge of vertex {} do not cycle", vertex);
			}
		}

		println!("Invariants passed");
	}

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