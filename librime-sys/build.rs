use std::env;
use std::path::PathBuf;

const DEFAULT_INCLUDE_DIR: &str = "/usr/include";
const DEFAULT_LIB_DIR: &str = "/usr/lib";

fn main() {
    let mut include_dir = DEFAULT_INCLUDE_DIR.to_owned();
    let mut lib_dir = DEFAULT_LIB_DIR.to_owned();

    if let Ok(e) = env::var("RIME_INCLUDE_DIR") {
        include_dir = e;
    }
    if let Ok(e) = env::var("RIME_LIB_DIR") {
        lib_dir = e;
    }

    println!("cargo:rustc-link-search={}", lib_dir);

    println!("cargo:rustc-link-lib=rime");

    let bindings = bindgen::Builder::default()
        .header(
            PathBuf::from(include_dir)
                .join("rime_api.h")
                .to_string_lossy(),
        )
        .header("./include/keycodes.h")
        .header("./include/modifiers.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
