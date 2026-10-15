use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));

    for (source_path, output_name) in [
        ("src/shaders/pyraminx.vert.wgsl", "pyraminx.vert.spv"),
        ("src/shaders/pyraminx.frag.wgsl", "pyraminx.frag.spv"),
    ] {
        println!("cargo:rerun-if-changed={source_path}");
        let source = fs::read_to_string(source_path).expect("read shader source");
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("validate shader");
        let words =
            naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
                .expect("compile shader to SPIR-V");
        let bytes = words
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        fs::write(out_dir.join(output_name), bytes).expect("write SPIR-V");
    }
}
