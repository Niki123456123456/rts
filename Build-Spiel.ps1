param([switch]$ServerChanged)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $env:CARGO_PROFILE_DEV_DEBUG = '0'
    $env:SPACETIMEDB_URI = 'http://127.0.0.1:18766'
    $env:SPACETIMEDB_DATABASE = 'stronghold-local'
    if ($ServerChanged) {
        cargo +stable build --manifest-path server/Cargo.toml --locked --release --target wasm32-unknown-unknown
        if ($LASTEXITCODE -ne 0) { throw 'Server-Build fehlgeschlagen.' }
        & ./runtime/spacetimedb-cli.exe --config-path server/.spacetime-local/cli.toml generate --lang rust --out-dir game/src/module_bindings --bin-path server/target/wasm32-unknown-unknown/release/stronghold_server.wasm
        if ($LASTEXITCODE -ne 0) { throw 'Rust-Anbindung konnte nicht erzeugt werden.' }
        & ./runtime/spacetimedb-cli.exe --config-path server/.spacetime-local/cli.toml generate --lang typescript --out-dir game/web/src/module_bindings --bin-path server/target/wasm32-unknown-unknown/release/stronghold_server.wasm
        if ($LASTEXITCODE -ne 0) { throw 'Browser-Anbindung konnte nicht erzeugt werden.' }
        if (-not (Test-Path game/web/node_modules)) {
            npm --prefix game/web ci --no-audit --no-fund
            if ($LASTEXITCODE -ne 0) { throw 'Netzwerk-Buildwerkzeuge fehlen.' }
        }
        npm --prefix game/web run check
        if ($LASTEXITCODE -ne 0) { throw 'Netzwerk-Prüfung fehlgeschlagen.' }
        npm --prefix game/web run build
        if ($LASTEXITCODE -ne 0) { throw 'Netzwerk-Build fehlgeschlagen.' }
        Copy-Item game/web/dist/network.js public/network.js -Force
        & ./runtime/spacetimedb-cli.exe --config-path server/.spacetime-local/cli.toml publish --server http://127.0.0.1:18766 --bin-path server/target/wasm32-unknown-unknown/release/stronghold_server.wasm stronghold-local --yes
        if ($LASTEXITCODE -ne 0) { throw 'Lokaler Server konnte nicht aktualisiert werden.' }
    }
    cargo +stable build --manifest-path game/Cargo.toml --locked --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'Spiel-Build fehlgeschlagen.' }
    & ./runtime/wasm-bindgen.exe --target web --out-name stronghold_game --out-dir public game/target/wasm32-unknown-unknown/debug/stronghold-game.wasm
    if ($LASTEXITCODE -ne 0) { throw 'Browser-Export fehlgeschlagen.' }
    Copy-Item game/web/index.html public/index.html -Force
    Copy-Item -Path game/assets/* -Destination public/assets -Recurse -Force
    Write-Host 'Spiel aktualisiert: http://127.0.0.1:18765/'
} finally { Pop-Location }
