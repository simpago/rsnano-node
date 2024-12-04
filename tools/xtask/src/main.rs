use duct::cmd;
use glob::glob;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

fn main() -> anyhow::Result<()> {
    coverage()
}

const HTML_TARGET_DIR: &str = "../build/coverage/html";
const LCOV_FILE: &str = "../build/coverage/tests.lcov";

pub fn coverage() -> anyhow::Result<()> {
    fs_extra::dir::remove(HTML_TARGET_DIR).map_err(anyhow::Error::msg)?;
    create_dir_all(HTML_TARGET_DIR)?;

    println!("=== running coverage ===");
    cmd!("cargo", "test", "--lib", "-q")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "cargo-test-%p-%m.profraw")
        .run()?;
    println!("ok.");

    println!("=== generating reports ===");
    run_gcov("html", HTML_TARGET_DIR)?;
    run_gcov("lcov", LCOV_FILE)?;
    println!("ok.");

    println!("=== cleaning up ===");
    clean_files("**/*.profraw")?;
    println!("ok.");
    println!("report location: {HTML_TARGET_DIR}");

    Ok(())
}

fn run_gcov(fmt: &str, file: &str) -> Result<(), anyhow::Error> {
    cmd!(
        "grcov",
        ".",
        "--binary-path",
        "../build/cargo/debug/deps",
        "-s",
        "..",
        "-t",
        fmt,
        "--ignore-not-existing",
        "--ignore",
        "rust/ffi/*",
        "--ignore",
        "../*",
        "--ignore",
        "/*",
        "--ignore",
        "rust/xtask/*",
        "--ignore",
        "*/src/tests/*",
        "-o",
        file,
    )
    .run()?;
    Ok(())
}

pub fn clean_files(pattern: &str) -> anyhow::Result<()> {
    let files: Result<Vec<PathBuf>, _> = glob(pattern)?.collect();
    files?.iter().try_for_each(remove_file)
}

pub fn remove_file<P>(path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    fs_extra::file::remove(path).map_err(anyhow::Error::msg)
}
