use std::{env, fs::File, io::Write, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut tests = File::create(&dest_path).unwrap();
    for entry in Path::new("tests").join("integration_tests").read_dir().expect("failed to read tests/integration tests") {
        let entry = entry.expect("failed to read dir entry");
        assert!(entry.file_type().unwrap().is_file());
        let test_path = entry.path();
        let test_name = test_path.file_stem().expect("no file stem").to_os_string().into_string().expect("file name not valid utf8");

        let content = format!(r#"
#[test]
fn {test_name}() {{
    run_test("{test_path}");
}}
"#, test_name = test_name.replace("-", "_"), test_path = test_path.display());

        tests.write_all(content.as_bytes()).expect("failed to write test");

        println!("cargo:rerun-if-changed={}", entry.path().display());
    }

    println!("cargo:rustc-env=GENERATED_TESTS={}", dest_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}
