include!(env!("GENERATED_TESTS"));

fn run_test(test_path: &str) {
    let mut cmd = std::process::Command::new("bootimage");
    cmd.arg("tester");
    cmd.arg(test_path);
    cmd.arg("--target");
    cmd.arg("../x86_64-bootimage-example-kernels.json");
    let output = cmd.output().expect("failed to run bootimage");
    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("test failed");
    }
}