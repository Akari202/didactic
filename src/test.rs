use std::path::PathBuf;

use env_logger::Env;

fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test")
}

fn run_build() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("didactic=debug,warn"))
        .is_test(true)
        .init();
    crate::build::run_build(test_dir(), false)
}

fn built_file(path: &str) -> PathBuf {
    test_dir().join("dist").join(path)
}

#[test]
fn test_build() {
    assert!(run_build().is_ok(), "Build should succeed");
    assert!(built_file("index.html").exists(), "index.html should exist");
    assert!(built_file("style.css").exists(), "style.css should exist");
    assert!(built_file("rss.xml").exists(), "rss.xml should exist");
}
