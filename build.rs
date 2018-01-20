extern crate gl_generator;

use gl_generator::{Registry, Api, Profile, Fallbacks, StaticGenerator};

use std::env;
use std::fs::File;
use std::path::Path;
use std::io::Write;

const INDEX_HTML_TEMPLATE: &'static str = 
r##"<html>
	<head>
		<meta charset="utf-8"/>
		<meta name='viewport' content='width=device-width, initial-scale=1.0, maximum-scale=1.0, minimum-scale=1.0, user-scalable=no' />
		<meta name="apple-mobile-web-app-capable" content="yes"/>
		<meta name="mobile-web-app-capable" content="yes"/>

		<meta name="theme-color" content="#333"/>
		<meta name="msapplication-navbutton-color" content="#333"/>
		<meta name="apple-mobile-web-app-status-bar-style" content="#333"/>

		<style>
			* {
				margin: 0;
				padding: 0;
			}

			html, body {
				width: 100vw;
				height: 100vh;
				overflow: hidden;
			}

			canvas {
				overflow: hidden;
				display: block;
			}
		</style>
	</head>

	<body>
		<canvas id="canvas"></canvas>
		<script src="[[pkg_name]]/[[build_type]].js"></script>
	</body>
</html>"##;

fn main() {
	let dest = env::var("OUT_DIR").unwrap();
	let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();

	Registry::new(Api::Gles2, (2, 1), Profile::Core, Fallbacks::All, [])
		.write_bindings(StaticGenerator, &mut file)
		.unwrap();

	let profile = env::var("PROFILE").unwrap();

	let index_html = INDEX_HTML_TEMPLATE.to_string()
		.replace("[[build_type]]", &profile)
		.replace("[[pkg_name]]", env!("CARGO_PKG_NAME"));
		
	let dest = env::var("CARGO_MANIFEST_DIR").unwrap();
	let path = Path::new(&dest).join("index.html");
	let mut file = File::create(&path).unwrap();

	file.write_all(index_html.as_bytes()).unwrap();
}
