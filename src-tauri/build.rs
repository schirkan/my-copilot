fn main() {
    tauri_build::build();

    // Sidecar-Binary in den Resource-Pfad kopieren, damit
    // `tauri-plugin-shell` es in `tauri dev` (resource_dir = target/debug)
    // und in `tauri build` (resource_dir = target/release) findet.
    //
    // Quelle: src-tauri/binaries/copilot-<TARGET_TRIPLE>.exe
    // Ziel:   <OUT_DIR>/../../binaries/copilot-<TARGET_TRIPLE>.exe
    //         (durch tauri_build::build() wird OUT_DIR auf
    //          target/<profile>/build/my-copilot-XXX/out gesetzt,
    //          also ist ../../binaries/ = target/<profile>/binaries/)
    copy_sidecar();
}

/// Kopiert die Copilot-CLI-Sidecar-Binary nach
/// `target/<profile>/binaries/`, wo Tauri 2 sie zur Laufzeit sucht.
#[cfg(windows)]
fn copy_sidecar() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = match std::env::var("OUT_DIR") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            println!("cargo:warning=OUT_DIR nicht gesetzt; ueberspringe Sidecar-Kopie");
            return;
        }
    };
    // OUT_DIR = target/<profile>/build/my-copilot-<hash>/out
    //   -> ../../binaries/ = target/<profile>/binaries/
    let target_binaries = out_dir
        .ancestors()
        .nth(3) // OUT_DIR -> build -> target
        .map(|p| p.join("binaries"))
        .unwrap_or_else(|| manifest_dir.join("target").join("binaries"));

    let triple = current_triple();
    let exe_name = format!("copilot-{}.exe", triple);
    let src = manifest_dir.join("binaries").join(&exe_name);
    let dst = target_binaries.join(&exe_name);

    if !src.exists() {
        println!(
            "cargo:warning=Sidecar-Binary nicht gefunden: {} (BYOK-Chat \
             wird fehlschlagen bis sie platziert wird)",
            src.display()
        );
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&target_binaries) {
        println!("cargo:warning=kann binaries/ nicht anlegen: {}", e);
        return;
    }

    // Nur kopieren, wenn Quelle neuer ist oder Ziel fehlt.
    let needs_copy = match (src.metadata(), dst.metadata()) {
        (Ok(s), Ok(d)) => s.modified().ok() > d.modified().ok(),
        (Ok(_), Err(_)) => true,
        _ => false,
    };
    if needs_copy {
        if let Err(e) = std::fs::copy(&src, &dst) {
            println!("cargo:warning=Sidecar {} -> {} fehlgeschlagen: {}",
                src.display(), dst.display(), e);
            return;
        }
        println!("cargo:rerun-if-changed={}", src.display());
        println!(
            "cargo:info=Sidecar kopiert: {} -> {}",
            src.display(),
            dst.display()
        );
    }

    // Immer neu auswerten, wenn sich die Binary aendert.
    println!("cargo:rerun-if-changed={}", src.display());
}

#[cfg(not(windows))]
fn copy_sidecar() {}

#[cfg(windows)]
fn current_triple() -> &'static str {
    // Standardmaessig host-Triple; fuer Cross-Builds wuerde man hier
    // env!("TARGET") auswerten, aber `cargo` selbst laeuft nativ,
    // also passt das fuer unseren dev/release-Fall.
    "x86_64-pc-windows-msvc"
}