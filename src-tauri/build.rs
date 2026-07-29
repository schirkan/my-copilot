fn main() {
    // tauri_build::build() laedt das Tauri-Build-Skript (Icons, Capabilities,
    // Schema-Generierung, etc.).
    //
    // Die Copilot-CLI-Binary wird vom `github-copilot-sdk` mit dem
    // `bundled-cli`-Feature automatisch beim Kompilieren heruntergeladen
    // und nach `target/<profile>/copilot.exe` entpackt.
    //
    // Damit `bundle.externalBin` (= "binaries/copilot-<triple>.exe") sie
    // beim `tauri build` findet, kopieren wir sie im `beforeBuildCommand`-
    // Hook (siehe tauri.conf.json) -- das laeuft NACH `cargo build` und damit
    // NACHDEM das SDK die CLI entpackt hat.
    tauri_build::build();
}