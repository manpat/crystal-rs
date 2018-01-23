#![allow(dead_code)]

use std::mem::transmute;
use std::ffi::CString;
use std::ptr;

use common::Vec2i;

use ::MainContext;

#[repr(C)]
struct EmscriptenMouseEvent {
	ts: f64,
	_screen: [i32; 2],
	x: i32, y: i32,
	_modifiers: [i32; 4],
	button: u16,
	_buttons: u16,

	dx: i32, dy: i32,
	// ... I don't care about the rest of these fields
}

#[repr(C)]
#[derive(Debug)]
struct EmscriptenKeyboardEvent {
	key: [u8; 32],
	code: [u8; 32],
	location: u32,

	ctrl_key: i32,
	shift_key: i32,
	alt_key: i32,
	meta_key: i32,

	repeat: i32,
	
	char_value: [u8; 32],
	char_code: u32,
	key_code: u32,
	which: u32,
}

#[repr(C)]
struct EmscriptenTouchPoint {
	id: i32,
	_screen: [i32; 2],
	x: i32, y: i32,
	_page: [i32; 2],
	is_changed: i32,

	_on_target: i32,
	_target: [i32; 2],
	_canvas: [i32; 2],
}

#[repr(C)]
struct EmscriptenTouchEvent {
	num_touches: i32,
	_modifiers: [i32; 4],
	touches: [EmscriptenTouchPoint; 32],
}

#[repr(C)]
struct EmscriptenPointerlockChangeEvent {
	active: i32,
	// ..
}

#[repr(C)]
pub struct EmscriptenWebGLContextAttributes {
	pub alpha: i32, // NOTE: this enables alpha blending of the *canvas itself*
	pub depth: i32,
	pub stencil: i32,
	pub antialias: i32,
	pub premultiplied_alpha: i32,
	pub preserve_drawing_buffer: i32,
	pub prefer_low_power_to_high_performance: i32,
	pub fail_if_major_performance_caveat: i32,

	pub major_version: i32,
	pub minor_version: i32,

	pub enable_extensions_by_default: i32,
}

pub const RESULT_SUCCESS: i32             =  0;
pub const RESULT_DEFERRED: i32            =  1;
pub const RESULT_NOT_SUPPORTED: i32       = -1;
pub const RESULT_FAILED_NOT_DEFERRED: i32 = -2;
pub const RESULT_INVALID_TARGET: i32      = -3;
pub const RESULT_UNKNOWN_TARGET: i32      = -4;
pub const RESULT_INVALID_PARAM: i32       = -5;
pub const RESULT_FAILED: i32              = -6;
pub const RESULT_NO_DATA: i32             = -7;
pub const RESULT_TIMED_OUT: i32           = -8;

pub type EmWebGLContext = i32;
pub type EmSocketCallback = extern fn(fd: i32, ud: *mut u8);
type EmPointerLockChangeCallback = extern fn(etype: i32, evt: *const EmscriptenPointerlockChangeEvent, ud: *mut u8) -> i32;
type EmMouseCallback = extern fn(etype: i32, evt: *const EmscriptenMouseEvent, ud: *mut u8) -> i32;
type EmTouchCallback = extern fn(etype: i32, evt: *const EmscriptenTouchEvent, ud: *mut u8) -> i32;
type EmKeyCallback = extern fn(etype: i32, evt: *const EmscriptenKeyboardEvent, ud: *mut u8) -> i32;
type EmArgCallback = extern fn(ud: *mut u8);

#[allow(improper_ctypes)]
extern {
	fn emscripten_set_main_loop_arg(func: extern fn(arg: *mut u8), arg: *mut u8, fps: i32, simulate_infinite_loop: i32);
	fn emscripten_exit_with_live_runtime();

	pub fn emscripten_set_socket_open_callback(ud: *mut u8, callback: EmSocketCallback);
	pub fn emscripten_set_socket_close_callback(ud: *mut u8, callback: EmSocketCallback);
	pub fn emscripten_set_socket_message_callback(ud: *mut u8, callback: EmSocketCallback);

	fn emscripten_set_pointerlockchange_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmPointerLockChangeCallback);

	fn emscripten_set_mousedown_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmMouseCallback);
	fn emscripten_set_mouseup_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmMouseCallback);
	fn emscripten_set_mousemove_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmMouseCallback);

	fn emscripten_set_touchstart_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmTouchCallback);
	fn emscripten_set_touchend_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmTouchCallback);
	fn emscripten_set_touchmove_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmTouchCallback);
	fn emscripten_set_touchcancel_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmTouchCallback);

	fn emscripten_set_keydown_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmKeyCallback);
	fn emscripten_set_keyup_callback(target: *const u8, ud: *mut u8, useCapture: i32, cb: EmKeyCallback);

	fn emscripten_request_pointerlock(target: *const u8, defer: i32) -> i32;
	fn emscripten_request_fullscreen(target: *const u8, defer: i32) -> i32;

	pub fn emscripten_async_call(callback: EmArgCallback, ud: *mut u8, millis: i32);
	pub fn emscripten_asm_const_int(s: *const u8, ...) -> i32;

	pub fn emscripten_webgl_init_context_attributes(attribs: *mut EmscriptenWebGLContextAttributes);
	pub fn emscripten_webgl_create_context(target: *const i8, attribs: *const EmscriptenWebGLContextAttributes) -> EmWebGLContext;
	pub fn emscripten_webgl_make_context_current(context: EmWebGLContext) -> i32;
	pub fn emscripten_webgl_destroy_context(context: EmWebGLContext) -> i32;
	pub fn emscripten_webgl_get_current_context() -> EmWebGLContext;
}

pub trait Interop {
	fn as_int(self, _: &mut Vec<CString>) -> i32;
}

impl Interop for i32 {
	fn as_int(self, _: &mut Vec<CString>) -> i32 {
		return self;
	}
}

impl<'a> Interop for &'a str {
	fn as_int(self, arena: &mut Vec<CString>) -> i32 {
		let c = CString::new(self).unwrap();
		let ret = c.as_ptr() as i32;
		arena.push(c);
		return ret;
	}
}

impl<'a> Interop for *const u8 {
	fn as_int(self, _: &mut Vec<CString>) -> i32 {
		return self as i32;
	}
}

#[macro_export]
macro_rules! js {
	( ($( $x:expr ),*) $y:expr ) => {
		{
			use std::ffi::CString;
			let mut arena: Vec<CString> = Vec::new();
			#[allow(dead_code)]
			const LOCAL: &'static [u8] = $y;
			unsafe { ::ems::emscripten_asm_const_int(&LOCAL[0] as *const _ as *const u8, $(::ems::Interop::as_int($x, &mut arena)),*) }
		}
	};
	( $y:expr ) => {
		{
			#[allow(dead_code)]
			const LOCAL: &'static [u8] = $y;
			unsafe { ::ems::emscripten_asm_const_int(&LOCAL[0] as *const _ as *const u8) }
		}
	};
}


pub fn register_callbacks(ctx: *mut MainContext) {
	unsafe {
		emscripten_set_touchstart_callback(ptr::null(), ctx as *mut u8, 1, on_touch_down);
		emscripten_set_touchend_callback(ptr::null(), ctx as *mut u8, 1, on_touch_up);
		emscripten_set_touchmove_callback(ptr::null(), ctx as *mut u8, 1, on_touch_move);
		emscripten_set_touchcancel_callback(ptr::null(), ctx as *mut u8, 1, on_touch_up);

		emscripten_set_mousedown_callback(ptr::null(), ctx as *mut u8, 0, on_mouse_down);
		emscripten_set_mouseup_callback(ptr::null(), ctx as *mut u8, 0, on_mouse_up);
		emscripten_set_mousemove_callback(ptr::null(), ctx as *mut u8, 0, on_mouse_move);

		emscripten_set_main_loop_arg(on_update, ctx as *mut u8, 0, 1);
	}
}

extern fn on_update(ud: *mut u8) {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };

	ctx.on_update();
	ctx.on_render();
}

extern fn on_touch_down(_: i32, ev: *const EmscriptenTouchEvent, ud: *mut u8) -> i32 {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };
	let ev = unsafe { &*ev };

	for i in 0..ev.num_touches {
		let touch = &ev.touches[i as usize];
		if touch.is_changed > 0 {
			ctx.on_touch_down(touch.id as u32, Vec2i::new(touch.x, touch.y));
		}
	}

	1
}

extern fn on_touch_up(_: i32, ev: *const EmscriptenTouchEvent, ud: *mut u8) -> i32 {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };
	let ev = unsafe { &*ev };

	ctx.is_touch_input = true;

	for i in 0..ev.num_touches {
		let touch = &ev.touches[i as usize];
		if touch.is_changed > 0 {
			ctx.on_touch_up(touch.id as u32);
		}
	}

	1
}

extern fn on_touch_move(_: i32, ev: *const EmscriptenTouchEvent, ud: *mut u8) -> i32 {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };
	let ev = unsafe { &*ev };

	for i in 0..ev.num_touches {
		let touch = &ev.touches[i as usize];
		if touch.is_changed > 0 {
			ctx.on_touch_move(touch.id as u32, Vec2i::new(touch.x, touch.y));
		}
	}

	1
}


extern fn on_mouse_down(_: i32, ev: *const EmscriptenMouseEvent, ud: *mut u8) -> i32 {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };
	let ev = unsafe { &*ev };

	if ctx.is_touch_input { return 0 }

	ctx.on_touch_down(0, Vec2i::new(ev.x, ev.y));

	0
}

extern fn on_mouse_up(_: i32, _ev: *const EmscriptenMouseEvent, ud: *mut u8) -> i32 {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };
	// let ev = unsafe { &*ev };

	if ctx.is_touch_input { return 0 }

	ctx.on_touch_up(0);

	0
}

extern fn on_mouse_move(_: i32, ev: *const EmscriptenMouseEvent, ud: *mut u8) -> i32 {
	let ctx: &mut MainContext = unsafe{ transmute(ud) };
	let ev = unsafe { &*ev };

	if ctx.is_touch_input { return 0 }

	ctx.on_touch_move(0, Vec2i::new(ev.x, ev.y));

	0
}
