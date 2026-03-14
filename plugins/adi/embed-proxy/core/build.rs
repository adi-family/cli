use std::path::PathBuf;
use typespec_api::codegen::{rust::RustAdiServiceConfig, Generator, Language, Side};

fn main() {
    let api_tsp = "../api.tsp";
    println!("cargo:rerun-if-changed={api_tsp}");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = std::env::var("OUT_DIR").unwrap();

    let source = std::fs::read_to_string(api_tsp).expect("read api.tsp");
    let file = typespec_api::parse(&source).expect("parse api.tsp");

    let gen_dir = PathBuf::from(&out_dir).join("embed_proxy_adi");
    let adi_config = RustAdiServiceConfig {
        types_crate: "crate".into(),
        cocoon_crate: "lib_adi_service".into(),
        service_name: "EmbedProxy".into(),
        ..Default::default()
    };

    Generator::new(&file, &gen_dir, "embed_proxy")
        .with_rust_adi_config(adi_config)
        .generate(Language::Rust, Side::AdiService)
        .expect("embed-proxy adi codegen failed");

    let adi_src = gen_dir.join("src/adi_service.rs");
    if adi_src.exists() {
        let content = std::fs::read_to_string(&adi_src).unwrap();
        std::fs::write(format!("{out_dir}/embed_proxy_adi_service.rs"), content).unwrap();
    }
}
