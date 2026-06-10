use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let root = workspace_root();
    doctest_book(&root).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
}

fn doctest_book(workspace: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let target = host_triple()?;

    // Build fabrique with --target so proc-macro (host) and library (target)
    // artifacts land in separate directories, avoiding E0464 duplicate rlib errors.
    run(Command::new("cargo").current_dir(workspace).args([
        "build",
        "-p",
        "fabrique",
        "--features",
        "sqlite,testing",
        "--target",
        &target,
    ]))?;

    // Assemble a clean deps dir: target rlibs (no duplicates) + proc-macro .so files.
    let doctests_deps = workspace.join("target/doctests-deps");
    if doctests_deps.exists() {
        fs::remove_dir_all(&doctests_deps)?;
    }
    fs::create_dir_all(&doctests_deps)?;

    let target_deps = workspace.join(format!("target/{target}/debug/deps"));
    for entry in fs::read_dir(&target_deps)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "rlib") {
            symlink(path.canonicalize()?, doctests_deps.join(path.file_name().unwrap()))?;
        }
    }

    let host_deps = workspace.join("target/debug/deps");
    for entry in fs::read_dir(&host_deps)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "so") {
            symlink(path.canonicalize()?, doctests_deps.join(path.file_name().unwrap()))?;
        }
    }

    run(Command::new("mdbook")
        .current_dir(workspace)
        .args(["build", "docs"]))?;

    run(Command::new("mdbook")
        .current_dir(workspace)
        .env("CARGO_MANIFEST_DIR", workspace.join("docs"))
        .args(["test", "docs", "-L", doctests_deps.to_str().unwrap()]))?;

    Ok(())
}

fn host_triple() -> Result<String, Box<dyn std::error::Error>> {
    let out = Command::new("rustc").arg("-vV").output()?;
    for line in String::from_utf8(out.stdout)?.lines() {
        if let Some(triple) = line.strip_prefix("host: ") {
            return Ok(triple.to_owned());
        }
    }
    Err("could not determine host triple from `rustc -vV`".into())
}

fn run(cmd: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("`{:?}` exited with {status}", cmd.get_program()).into());
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    let mut dir = env::current_dir().expect("cannot determine current directory");
    loop {
        if dir.join("Cargo.toml").exists() && dir.join(".git").exists() {
            return dir;
        }
        assert!(dir.pop(), "could not find workspace root");
    }
}
