use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let py_dir = "C:\\Users\\T03000\\AppData\\Local\\Programs\\Python\\Python312";
    let dll_src = PathBuf::from(py_dir).join("python312.dll");

    // Copy to target directory so the executable can load python312.dll directly
    if let Ok(out_dir) = env::var("OUT_DIR") {
        let out_path = PathBuf::from(out_dir);
        // Find the target/<profile> directory
        if let Some(target_dir) = out_path.ancestors().nth(3) {
            let target_dll = target_dir.join("python312.dll");
            if dll_src.exists() && !target_dll.exists() {
                let _ = fs::copy(&dll_src, &target_dll);
            }
        }
    }
}
