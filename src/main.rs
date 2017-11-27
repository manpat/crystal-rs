#![feature(const_fn)]
#![feature(link_args)]
#![feature(box_syntax)]
#![feature(ord_max_min)]
#![feature(slice_patterns)]
#![feature(inclusive_range_syntax)]

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
use rendering::framebuffer::{Framebuffer, FramebufferBuilder};
use rendering::*;

pub use resources::*;
pub use easing::*;
pub use math::*;

use rand::{random, Closed01, thread_rng, Rng};

pub fn rand_f32(range: f32) -> f32 {
	let Closed01(f) = random::<Closed01<f32>>();
	f * range
}

pub fn rand_vec2() -> Vec2 {
	Vec2::new(rand_f32(2.0) - 1.0, rand_f32(2.0) - 1.0)
}

pub fn rand_vec3() -> Vec3 {
	Vec3::new(rand_f32(2.0) - 1.0, rand_f32(2.0) - 1.0, rand_f32(2.0) - 1.0)
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
	shader_fb: Shader,
	shader_star: Shader,
	shader_color: Shader,
	shader_crystal: Shader,
	shader_line_fuzz: Shader,
	shader_star_compose: Shader,
	prev_frame: time::Instant,
	time: f64,

	cmbuilder: MeshBuilder,
	crystal_mesh: Mesh,
	crystal_mesh_lines: Mesh,
	crystal_targets: [Framebuffer; 2],
	crystal_refract_idx: f32,
	crystal_line_targets: [Framebuffer; 2],
	target_flip: usize,

	quad_mesh: Mesh,
	star_mesh: Mesh,
	star_target: Framebuffer,

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
			gl::Enable(gl::CULL_FACE);
			gl::Enable(gl::BLEND);

			gl::BlendEquation(gl::FUNC_ADD);
			gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
		}

		let mut star_mesh = Mesh::new();
		let mut star_builder = MeshBuilder::new();

		for _ in 0..500 {
			use rendering::mesh_builder::Vertex;

			let x = rand_f32(1.0);

			let dist = (1.0 - x) * 190.0 + 15.0;
			let color = x * 0.3 + 0.02;
			let point_size = x * 5.0 + 1.0;

			let info = Vec3::new(color, point_size, 0.0);

			star_builder.add_vert(Vertex::new_normal(rand_vec3().normalize() * dist, info));
		}

		star_builder.upload_to(&mut star_mesh);

		let mut quad_mesh = Mesh::new();
		let mut quad_builder = star_builder;

		{	use rendering::mesh_builder::Vertex;

			quad_builder.clear();
			quad_builder.add_quad(&[
				Vertex::new(Vec3::new(-1.0,-1.0, 0.0)),
				Vertex::new(Vec3::new( 1.0,-1.0, 0.0)),
				Vertex::new(Vec3::new( 1.0, 1.0, 0.0)),
				Vertex::new(Vec3::new(-1.0, 1.0, 0.0)),
			]);
		}

		quad_builder.upload_to(&mut quad_mesh);

		let mut star_target = FramebufferBuilder::new_unsized()
			.add_target()
			.finalize();

		star_target.get_target(0).unwrap().nearest();

		let crystal_targets = [
			FramebufferBuilder::new_unsized()
				.add_target()
				.add_depth()
				.finalize(),

			FramebufferBuilder::new_unsized()
				.add_target()
				.add_depth()
				.finalize(),
		];

		let crystal_line_targets = [
			FramebufferBuilder::new_unsized()
				.add_target()
				.finalize(),

			FramebufferBuilder::new_unsized()
				.add_target()
				.finalize(),
		];

		MainContext {
			viewport: Viewport::new(),
			shader_fb: Shader::new(FB_SHADER_VERT_SRC, FB_SHADER_FRAG_SRC),
			shader_star: Shader::new(STAR_SHADER_VERT_SRC, STAR_SHADER_FRAG_SRC),
			shader_color: Shader::new(BASIC_TRANSFORM_SHADER_VERT_SRC, COLOR_SHADER_FRAG_SRC),
			shader_crystal: Shader::new(CRYSTAL_SHADER_VERT_SRC, CRYSTAL_SHADER_FRAG_SRC),
			shader_line_fuzz: Shader::new(FB_SHADER_VERT_SRC, LINE_FUZZ_SHADER_FRAG_SRC),
			shader_star_compose: Shader::new(FB_SHADER_VERT_SRC, STAR_COMPOSE_SHADER_FRAG_SRC),
			prev_frame: time::Instant::now(),
			time: 0.0,

			cmbuilder: MeshBuilder::new(),
			crystal_mesh: Mesh::new(),
			crystal_mesh_lines: Mesh::new(),
			crystal_targets,
			crystal_refract_idx: 1.0,
			crystal_line_targets,
			target_flip: 0,

			quad_mesh,
			star_mesh,
			star_target,

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

		if self.touch_id.is_none() && self.touch_delta.length() < 0.0005 {
			self.touch_delta = self.touch_delta + rand_vec2() * 0.0001;

			// Sustain momentum
			let dir = self.touch_delta.normalize();
			if dir.x.is_finite() && dir.y.is_finite() {
				self.touch_delta = self.touch_delta + dir * 0.0002;
			}
		}

		if self.touch_id.is_none() {
			self.touch_delta = self.touch_delta * (1.0 - dt as f32 * 0.3);
		}

		self.fit_canvas();
		self.crystal_targets[0].resize(self.viewport.size);
		self.crystal_targets[1].resize(self.viewport.size);
		self.crystal_line_targets[0].resize(self.viewport.size);
		self.crystal_line_targets[1].resize(self.viewport.size);
		self.star_target.resize(self.viewport.size);

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

			let rot_diff = Quat::new(Vec3::new(0.0, 1.0, 0.0), -self.touch_delta.x * PI / 2.0)
				* Quat::new(Vec3::new(1.0, 0.0, 0.0), -self.touch_delta.y * PI / 2.0);

			let new_rotation = (self.rotation * rot_diff).normalize();

			let proj_mat = Mat4::perspective(PI/3.0, self.viewport.get_aspect(), 0.005, 1000.0);
			let trans_mat = Mat4::translate(Vec3::new(0.0, 0.0,-2.0));

			gl::EnableVertexAttribArray(0);
			gl::EnableVertexAttribArray(1);
			
			self.star_target.bind();
			self.shader_color.use_program();
			self.shader_color.set_proj(&Mat4::ident());
			self.shader_color.set_uniform_vec4("u_color", &Vec4::new(0.0, 0.0, 0.0, 0.15));
			self.quad_mesh.bind();
			self.quad_mesh.draw(gl::TRIANGLES);

			self.shader_star.use_program();
			self.shader_star.set_uniform_f32("u_time", self.time as f32);

			let max_star_steps = 100u32;

			for i in 0...max_star_steps {
				let a = i as f32 / max_star_steps as f32;
				let rotation = (self.rotation * (1.0 - a) + new_rotation * a).normalize();

				let view_mat = trans_mat * rotation.to_mat4();
				let view_proj = proj_mat * view_mat;

				self.shader_star.set_proj(&view_proj);
				self.star_mesh.bind();
				self.star_mesh.draw(gl::POINTS);
			}
			Framebuffer::unbind();

			self.rotation = new_rotation;

			let view_mat = trans_mat * new_rotation.to_mat4();
			let view_proj = proj_mat * view_mat;

			self.shader_crystal.use_program();
			self.shader_crystal.set_proj(&view_proj);
			self.shader_crystal.set_view(&view_mat);

			self.crystal_mesh.bind();
			gl::ClearColor(0.0, 0.0, 0.0, 0.0);

			self.crystal_targets[0].bind();
			gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

			gl::FrontFace(gl::CW);
			self.shader_crystal.set_uniform_vec3("u_color", &Vec3::new(0.9, 0.0, 0.5));
			self.crystal_mesh.draw(gl::TRIANGLES);

			self.crystal_targets[1].bind();
			gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

			gl::FrontFace(gl::CCW);
			self.shader_crystal.set_uniform_vec3("u_color", &Vec3::new(0.3, 0.0, 1.0));
			self.crystal_mesh.draw(gl::TRIANGLES);

			Framebuffer::unbind();

			{
				self.crystal_targets[0].get_target(0).unwrap().bind_to_slot(0);
				self.crystal_targets[1].get_target(0).unwrap().bind_to_slot(1);
				self.crystal_targets[0].get_depth().unwrap().bind_to_slot(2);
				self.crystal_targets[1].get_depth().unwrap().bind_to_slot(3);
				self.star_target.get_target(0).unwrap().bind_to_slot(4);

				self.shader_star_compose.use_program();
				self.shader_star_compose.set_proj(&proj_mat);
				self.shader_star_compose.set_uniform_mat("inv_proj", &proj_mat.inverse());
				self.shader_star_compose.set_uniform_f32("u_refractive_index", self.crystal_refract_idx);

				self.shader_star_compose.set_uniform_i32("u_color0", 0);
				self.shader_star_compose.set_uniform_i32("u_color1", 1);
				self.shader_star_compose.set_uniform_i32("u_depth0", 2);
				self.shader_star_compose.set_uniform_i32("u_depth1", 3);
				self.shader_star_compose.set_uniform_i32("u_bgcolor", 4);
				self.shader_star_compose.set_uniform_f32("u_time", self.time as f32);
				self.quad_mesh.bind();
				self.quad_mesh.draw(gl::TRIANGLES);

				Texture::unbind();
			}

			
			gl::Disable(gl::DEPTH_TEST);

			self.crystal_line_targets[self.target_flip].bind();
			
			self.shader_color.use_program();
			self.shader_color.set_proj(&view_proj);
			self.shader_color.set_uniform_vec4("u_color", &Vec4::new(0.9, 0.85, 0.87, 0.4));
			self.crystal_mesh_lines.bind();
			self.crystal_mesh_lines.draw(gl::LINES);
			Framebuffer::unbind();

			self.shader_line_fuzz.use_program();
			self.shader_line_fuzz.set_uniform_i32("u_color", 0);
			self.shader_line_fuzz.set_uniform_f32("u_aspect", self.viewport.get_aspect());
			self.shader_line_fuzz.set_uniform_f32("u_time", self.time as f32);

			self.crystal_line_targets[1 - self.target_flip].bind();
			self.crystal_line_targets[self.target_flip].get_target(0).unwrap().bind_to_slot(0);

			self.quad_mesh.bind();
			self.quad_mesh.draw(gl::TRIANGLES);
			Framebuffer::unbind();

			self.target_flip = 1 - self.target_flip;

			self.crystal_line_targets[self.target_flip].get_target(0).unwrap().bind_to_slot(0);
			self.shader_fb.use_program();
			self.quad_mesh.bind();
			self.quad_mesh.draw(gl::TRIANGLES);

			gl::Enable(gl::DEPTH_TEST);
		}
	}

	fn on_touch_down(&mut self, id: u32, pos: Vec2i) {
		if self.touch_id.is_some() {
			self.build_crystal();
			return
		}

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
		crystal.build_faces(&mut self.cmbuilder);
		self.cmbuilder.upload_to(&mut self.crystal_mesh);

		self.cmbuilder.clear();
		crystal.build_edges(&mut self.cmbuilder, 0.05);
		self.cmbuilder.upload_to(&mut self.crystal_mesh_lines);

		self.crystal_refract_idx = thread_rng().gen_range(1.0 / 4.0, 1.0 / 1.01);
	}
}