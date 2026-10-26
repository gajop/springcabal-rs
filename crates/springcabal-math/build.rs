use std::{env, path::PathBuf, process::Command};

fn run(command: &mut Command) {
    let status = command
        .status()
        .expect("Clang/LLVM is required to build springcabal-math (see its README)");
    assert!(
        status.success(),
        "building the engine-compatible math library failed"
    );
}

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let target = env::var("TARGET").expect("Cargo TARGET");
    let compiler = env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let archiver = env::var("AR").unwrap_or_else(|_| {
        let output = Command::new(&compiler)
            .arg("-print-prog-name=llvm-ar")
            .output()
            .expect("Clang is required to build springcabal-math (see its README)");
        assert!(
            output.status.success(),
            "Clang could not locate its LLVM archiver"
        );
        String::from_utf8(output.stdout)
            .expect("LLVM archiver path")
            .trim()
            .into()
    });
    let files = [
        "e_powf",
        "k_sinf",
        "k_cosf",
        "s_sinf",
        "s_cosf",
        "e_rem_pio2f",
        "k_rem_pio2f",
        "s_scalbnf",
    ];
    let mut objects = Vec::new();
    for name in files {
        let source = format!("csrc/{name}.cpp");
        let object = out.join(format!("{name}.o"));
        run(Command::new(&compiler)
            .args([
                "-target",
                &target,
                "-std=c++11",
                "-O2",
                "-fno-exceptions",
                "-fno-rtti",
                "-fno-fast-math",
                "-ffp-contract=off",
                "-fno-builtin",
                "-ffunction-sections",
                "-fdata-sections",
                "-c",
                &source,
                "-o",
            ])
            .arg(&object));
        objects.push(object);
        println!("cargo:rerun-if-changed={source}");
    }
    run(Command::new(&archiver)
        .arg("crs")
        .arg(out.join("libspringcabal_math.a"))
        .args(objects));
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=springcabal_math");
    println!("cargo:rerun-if-changed=csrc/control.h");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=AR");
}
