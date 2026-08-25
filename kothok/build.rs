fn main() {
    slint_build::compile_with_config(
        "ui/reader.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();

    let kc = kobo_core_tag();
    println!("cargo:rustc-env=KOBO_CORE_REV={kc}");
    println!("cargo:rerun-if-changed=Cargo.lock");
}

// The kobo-core version baked into the BUILD stamp so a deployed binary
// self-describes which dependency it was compiled with. Read from Cargo.lock,
// which pins the exact resolved version regardless of source -- registry, git,
// or path -- so the stamp never silently lags the dependency. KOBO_CORE_REV
// overrides if set (e.g. CI injecting a git rev).
fn kobo_core_tag() -> String {
    if let Ok(v) = std::env::var("KOBO_CORE_REV") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return v;
        }
    }
    let Ok(lock) = std::fs::read_to_string("Cargo.lock") else {
        return "?".to_string();
    };
    let mut want = false;
    for line in lock.lines() {
        let t = line.trim();
        if t == "[[package]]" {
            want = false;
        } else if t.starts_with("name = \"kobo-core\"") {
            want = true;
        } else if want {
            if let Some(rest) = t.strip_prefix("version =") {
                return rest.trim().trim_matches('"').to_string();
            }
        }
    }
    "?".to_string()
}
