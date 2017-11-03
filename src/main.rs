#![feature(const_fn)]
#![feature(link_args)]
#![feature(box_syntax)]
#![feature(ord_max_min)]
#![feature(slice_patterns)]

extern crate rand;

#[macro_use]
mod enums {
	macro_rules! match_enum {
		($v:expr, $p:pat) => {
			match $v {
				$p => true,
				_ => false,
			}
		}
	}
}

mod resources;
mod rendering;
mod easing;
mod math;

mod crystal;

#[macro_use] mod ems;

use std::time;

use rendering::mesh_builder::{MeshBuilder, Mesh};
use rendering::*;

pub use resources::*;
pub use easing::*;
pub use math::*;

use rand::{random, Closed01};

pub fn rand_f32 (range: f32) -> f32 {
	let Closed01(f) = random::<Closed01<f32>>();
	f * range
}

pub fn rand_vec2 (range: Vec2) -> Vec2 {
	Vec2::new(rand_f32(range.x), rand_f32(range.y))
}


fn main() {
	use std::mem::uninitialized;

	let ems_context_handle = unsafe {
		let mut attribs = uninitialized();
		ems::emscripten_webgl_init_context_attributes(&mut attribs);
		attribs.alpha = 0;
		attribs.stencil = 1;
		attribs.antialias = 1;
		attribs.preserve_drawing_buffer = 0;
		attribs.enable_extensions_by_default = 0;

		ems::emscripten_webgl_create_context(b"canvas\0".as_ptr() as _, &attribs)
	};

	match ems_context_handle {
		ems::RESULT_NOT_SUPPORTED => {
			panic!("WebGL not supported");
		}

		ems::RESULT_FAILED_NOT_DEFERRED => {
			panic!("WebGL context creation failed (FAILED_NOT_DEFERRED)");
		}

		ems::RESULT_FAILED => {
			panic!("WebGL context creation failed (FAILED)");
		}

		x if x < 0 => {
			panic!("WebGL context creation failed ({})", x);
		}

		_ => {}
	}

	if unsafe {ems::emscripten_webgl_make_context_current(ems_context_handle) != ems::RESULT_SUCCESS} {
		panic!("Failed to make webgl context current");
	}

	// js!{ b"document.addEventListener('contextmenu', function(e) { e.preventDefault(); return false; })\0" };

	let ctx = MainContext::new();
	ems::register_callbacks(Box::into_raw(box ctx));
}

pub struct MainContext {
	viewport: Viewport,
	shader: Shader,
	prev_frame: time::Instant,
	time: f64,

	cmbuilder: MeshBuilder,
	crystal_mesh: Mesh,
	crystal_mesh_points: Mesh,

	rotation: Quat,

	touch_id: Option<u32>,
	touch_start: Vec2i,
	touch_prev: Vec2i,

	touch_delta: Vec2,

	// ems info
	is_touch_input: bool,
	is_potential_tap: bool,
}

impl MainContext {
	fn new() -> Self {
		unsafe {
			gl::Enable(gl::DEPTH_TEST);

			// gl::Enable(gl::BLEND);
			// gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
			// gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ONE, gl::ZERO);
		}

		MainContext {
			viewport: Viewport::new(),
			shader: Shader::new(&MAIN_SHADER_VERT_SRC, &MAIN_SHADER_FRAG_SRC),
			prev_frame: time::Instant::now(),
			time: 0.0,

			cmbuilder: MeshBuilder::new(),
			crystal_mesh: Mesh::new(),
			crystal_mesh_points: Mesh::new(),

			rotation: Quat::from_raw(0.0, 0.0, 0.0, 1.0),

			touch_id: None,
			
			touch_start: Vec2i::zero(),
			touch_prev: Vec2i::zero(),
			touch_delta: Vec2::zero(),

			is_touch_input: false,
			is_potential_tap: false,
		}
	}

	fn on_update(&mut self) {
		let now = time::Instant::now();
		let diff = now - self.prev_frame;
		self.prev_frame = now;

		let udt = diff.subsec_nanos() / 1000;
		let dt = udt as f64 / 1000_000.0;

		self.time += dt;

		self.fit_canvas();

		if self.crystal_mesh.count < 3 {
			self.build_crystal();
		}
	}

	fn on_render(&mut self) {
		unsafe {
			let g = 0.03;
			gl::ClearColor(g, g, g, 1.0);
			gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

			let Vec2i{x: vw, y: vh} = self.viewport.size;
			gl::Viewport(0, 0, vw, vh);

			if self.touch_id.is_none() {
				self.touch_delta = self.touch_delta * (1.0 - 1.0/60.0);
			}

			self.rotation = self.rotation
				* Quat::new(Vec3::new(0.0, 1.0, 0.0), -self.touch_delta.x * PI / 2.0)
				* Quat::new(Vec3::new(1.0, 0.0, 0.0), -self.touch_delta.y * PI / 2.0);

			self.rotation = self.rotation.normalize();

			let view_proj = Mat4::perspective(PI/3.0, self.viewport.get_aspect(), 0.005, 10.0)
				* Mat4::translate(Vec3::new(0.0, 0.0,-2.0))
				* self.rotation.to_mat4();
			
			self.shader.use_program();
			self.shader.set_proj(&view_proj);

			gl::EnableVertexAttribArray(0);

			self.crystal_mesh.bind();
			self.crystal_mesh.draw(gl::LINES);
			
			self.crystal_mesh_points.bind();
			self.crystal_mesh_points.draw(gl::POINTS);
		}
	}

	fn on_touch_down(&mut self, id: u32, pos: Vec2i) {
		if self.touch_id.is_some() { return }

		self.touch_id = Some(id);
		self.touch_start = pos;
		self.touch_prev = pos;
		self.touch_delta = Vec2::zero();

		self.is_potential_tap = true;
	}

	fn on_touch_up(&mut self, id: u32) {
		if self.touch_id != Some(id) { return }
		self.touch_id = None;

		if self.is_touch_input && self.is_potential_tap {
			js! {{ b"Module.requestFullscreen(1, 1)\0" }};
		}
	}

	fn on_touch_move(&mut self, id: u32, pos: Vec2i) {
		if self.touch_id != Some(id) { return }

		if (pos - self.touch_start).length() > 5.0 {
			self.is_potential_tap = false;
		}

		let minor = self.viewport.size.x.min(self.viewport.size.y);

		let diff = pos - self.touch_prev;
		self.touch_delta = 0.9f32.ease_linear(self.touch_delta, diff.to_vec2() / Vec2::splat(minor as f32));
		self.touch_prev = pos;
	}

	fn fit_canvas(&mut self) {
		js! { b"Module.canvas = document.getElementById('canvas')\0" };

		let w = js! { b"return (Module.canvas.width = Module.canvas.style.width = window.innerWidth)\0" };
		let h = js! { b"return (Module.canvas.height = Module.canvas.style.height = window.innerHeight)\0" };

		self.viewport.size = Vec2i::new(w, h);
	}

	fn build_crystal(&mut self) {
		use crystal::Crystal;

		println!("Generating crystal");

		let mut crystal = Crystal::new();
		crystal.radius = 0.5;

		crystal.generate();
		self.cmbuilder.clear();
		crystal.build_with(&mut self.cmbuilder);
		self.cmbuilder.upload_to(&mut self.crystal_mesh);

		self.cmbuilder.clear();
		crystal.build_points_with(&mut self.cmbuilder);
		self.cmbuilder.upload_to(&mut self.crystal_mesh_points);
	}
}