use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../worker/streamrecorder_worker");
    let mut entries: Vec<_> = fs::read_dir(&root)
        .expect("worker 源文件目录不存在")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "py"))
        .collect();
    entries.sort();
    let mut table = String::from(
        "pub static WORKER_FILES: &[(&str, &[u8])] = &[
",
    );
    for path in entries {
        println!("cargo:rerun-if-changed={}", path.display());
        table.push_str(&format!(
            "({:?}, include_bytes!({:?})),
",
            path.file_name().unwrap().to_str().unwrap(),
            path.canonicalize().unwrap().to_str().unwrap()
        ));
    }
    table.push_str(
        "];
",
    );
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("worker_assets.rs"),
        table,
    )
    .unwrap();
    tauri_build::build();
}
