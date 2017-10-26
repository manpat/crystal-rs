// #[link_args = "-s FULL_ES2=1"]
// extern {}

pub mod gl {
	#![allow(non_upper_case_globals)]
	include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

	pub fn pls_make_buffer() -> u32 {
		unsafe {
			let mut vbo = 0u32;
			GenBuffers(1, &mut vbo);
			vbo
		}
	}
}

pub mod types;
pub mod shader;
pub mod texture;

pub mod mesh_builder;

pub use self::types::*;
pub use self::shader::*;
pub use self::texture::*;
