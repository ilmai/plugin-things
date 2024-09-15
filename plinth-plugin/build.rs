use std::env;
use std::path::PathBuf;

fn main() {
    #[cfg(target_os="macos")]
    generate_plinth_bindings();
}

#[allow(dead_code)]
fn generate_plinth_bindings() {
    let bindings = bindgen::Builder::default()
        .header("include/plinth_auv3.h")
        .blocklist_type("AURenderEvent")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
