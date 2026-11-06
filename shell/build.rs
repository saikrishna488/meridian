fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let architecture = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let bundle = root.join("native/prebuilt").join(&architecture);
    let manifest = bundle.join("manifest.sha256");
    for name in ["font", "graphics", "performance", "audio"] {
        println!("cargo:rerun-if-changed=native/engines/{name}.cpp");
    }
    println!("cargo:rerun-if-changed=native/engines/engines.h");
    println!("cargo:rerun-if-changed={}", bundle.display());
    let native_host = std::env::var("HOST").ok() == std::env::var("TARGET").ok();
    let verified = native_host && manifest.is_file() && std::process::Command::new("sha256sum")
        .args(["--check", "--status"]).arg(&manifest).current_dir(&root).status().is_ok_and(|status| status.success());
    if verified {
        println!("cargo:rustc-link-search=native={}", bundle.display());
        for name in ["font", "gpu", "cpu", "audio"] { println!("cargo:rustc-link-lib=static=meridian-{name}"); }
        println!("cargo:rustc-link-lib=dylib=stdc++");
    } else {
        // Unsupported/stale bundles use source, never an incompatible architecture.
        let mut build = cc::Build::new();
        build.cpp(true).std("c++17").opt_level_str("s").warnings(true).flag_if_supported("-Wextra");
        for library in ["fontconfig", "freetype2", "pangocairo"] {
            let package = pkg_config::Config::new().cargo_metadata(false).probe(library)
                .expect("Engines require Fontconfig, FreeType and Pango/Cairo development packages");
            for path in package.include_paths { build.include(path); }
        }
        for name in ["font", "graphics", "performance", "audio"] { build.file(format!("native/engines/{name}.cpp")); }
        build.compile("meridian_engines");
    }
    // Backend libraries follow our static archives for --as-needed linkers.
    for library in ["fontconfig", "freetype2", "pangocairo"] {
        pkg_config::Config::new().probe(library).expect("Native engine backend unavailable");
    }
}
