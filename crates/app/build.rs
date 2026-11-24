//! Build-only Windows resource tooling; no bundled native browser implementation.
use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=resources/opensequoia.rc");
    println!("cargo:rerun-if-changed=resources/opensequoia.manifest");
    println!("cargo:rerun-if-env-changed=WindowsSdkDir");
    if !env::var("TARGET").is_ok_and(|target| target.ends_with("windows-msvc")) {
        return;
    }
    let sdk = env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10"));
    let mut versions: Vec<_> = fs::read_dir(sdk.join("bin"))
        .expect("Windows SDK bin directory must exist for executable metadata")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    let host = env::var("HOST").expect("Cargo HOST");
    let architecture = if host.starts_with("aarch64") {
        "arm64"
    } else {
        "x64"
    };
    let compiler = versions
        .into_iter()
        .rev()
        .map(|path| path.join(architecture).join("rc.exe"))
        .find(|path| path.is_file())
        .expect("Windows SDK rc.exe required; set WindowsSdkDir to its SDK root");
    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR")).join("opensequoia.res");
    let version = compiler
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.file_name())
        .expect("SDK version directory");
    let includes = sdk.join("Include").join(version);
    let result = Command::new(&compiler)
        .args(["/nologo", "/I"])
        .arg(includes.join("um"))
        .arg("/I")
        .arg(includes.join("shared"))
        .arg("/I")
        .arg(includes.join("ucrt"))
        .arg("/fo")
        .arg(&output)
        .arg("resources/opensequoia.rc")
        .status()
        .expect("run Windows resource compiler");
    assert!(
        result.success(),
        "Windows metadata resource compilation failed"
    );
    println!("cargo:rustc-link-arg-bin=opensequoia={}", output.display());
}
