use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::string::ToString;

fn out_dir() -> PathBuf {
  PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR env var"))
}

fn source_path() -> PathBuf {
  out_dir().join("x264-stable")
}

fn install_prefix() -> PathBuf {
  out_dir().join("build")
}

fn build_x264() {
  let source_path = source_path();

  let mut prefix_arg = OsString::from("--prefix=");
  prefix_arg.push(&install_prefix());

  let result = Command::new("bash")
    .arg("configure")
    .arg(prefix_arg)
    .arg("--disable-cli")
    .arg("--enable-static")
    .current_dir(&source_path)
    .status()
    .unwrap();

  if !result.success() {
    panic!("failed to configure x264");
  }

  let result = Command::new("make")
    .arg("-j")
    .arg(num_cpus::get().to_string())
    .arg("all")
    .arg("install")
    .current_dir(&source_path)
    .status()
    .unwrap();

  if !result.success() {
    panic!("Failed to build x264");
  }
}

pub const STATIC_LIBS: &[(&str, &str)] = &[("x264", "./libx264.a")];

pub const HEADERS: &[&str] = &["x264.h"];

fn build() {
  Command::new("mkdir")
    .args(&["-p", out_dir().join("x264-stable").to_str().unwrap()])
    .status()
    .unwrap();
  Command::new("cp")
    .args(&[
      "-r",
      "x264-stable/",
      out_dir().join("x264-stable").to_str().unwrap(),
    ])
    .status()
    .unwrap();

  assert!(source_path().exists());

  build_x264();

  println!("cargo:rustc-link-search=native={}", {
    install_prefix().join("lib").to_str().unwrap()
  });
  for (name, _) in STATIC_LIBS {
    println!("cargo:rustc-link-lib=static={}", name);
  }
  let codegen = |file_name: &str, headers: &[&str]| {
    let codegen = bindgen::Builder::default();
    let codegen = codegen.header("include/prelude.h");
    let codegen = headers.iter().fold(
      codegen,
      |codegen: bindgen::Builder, path: &&str| -> bindgen::Builder {
        let path: &str = path.clone();
        let path: PathBuf = install_prefix()
          .parent()
          .unwrap()
          .join("x264-stable")
          .join(path);
        let path: &str = path.to_str().expect("PathBuf to str");
        if !PathBuf::from(path).exists() {
          panic!("{} not existed", path);
        }
        codegen.header(path)
      },
    );
    codegen
      .generate_comments(true)
      .generate()
      .expect("Unable to generate bindings")
      .write_to_file(out_dir().join(file_name))
      .expect("Couldn't write bindings!");
  };
  codegen("bindings_x264.rs", HEADERS);
  // CARGO METADATA
  println!(
    "cargo:libs={}",
    install_prefix().join("lib").to_str().unwrap()
  );
  println!(
    "cargo:pkgconfig={}",
    install_prefix()
      .join("lib")
      .join("pkgconfig")
      .to_str()
      .unwrap()
  );
}

fn main() {
  build();
}
