use std::path::PathBuf;
use typespec_api::codegen::{rust::RustAdiServiceConfig, Generator, Language, Side};

fn main() {
    let api_tsp = "../api.tsp";
    println!("cargo:rerun-if-changed={api_tsp}");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = std::env::var("OUT_DIR").unwrap();

    let source = std::fs::read_to_string(api_tsp).expect("read api.tsp");
    let file = typespec_api::parse(&source).expect("parse api.tsp");

    let cred_dir = PathBuf::from(&out_dir).join("credentials_adi");
    let adi_config = RustAdiServiceConfig {
        types_crate: "crate".into(),
        cocoon_crate: "lib_adi_service".into(),
        service_name: "Credentials".into(),
        ..Default::default()
    };

    Generator::new(&file, &cred_dir, "credentials")
        .with_rust_adi_config(adi_config)
        .generate(Language::Rust, Side::AdiService)
        .expect("credentials adi codegen failed");

    let adi_src = cred_dir.join("src/adi_service.rs");
    if adi_src.exists() {
        let content = std::fs::read_to_string(&adi_src).unwrap();
        std::fs::write(format!("{out_dir}/credentials_adi_service.rs"), content).unwrap();
    }
}
