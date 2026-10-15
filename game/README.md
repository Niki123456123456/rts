# Stronghold

## Lokal spielen

Das Spiel verbindet sich standardmäßig mit dem gehosteten Multiplayer-Server auf
Maincloud (`stronghold-niki123456123456`). Einen eigenen lokalen Server kannst du
wie in [server/README.md](../server/README.md) beschrieben starten.

```sh
bash server/generate-bindings.sh
cd game
cargo run
```

Beim Start erscheint die Stronghold-Lobby mit Verbindungsanzeige. Server-Einstellungen
sind unter **Connection settings** eingeklappt. Gib einen Titel ein und klicke auf **Create
Game**, oder bewege die Maus über ein vorhandenes Spiel und klicke auf **Join**.
Jeder Spieler erhält 20 Einheiten in seiner Farbe und kann nur seine eigenen
Einheiten auswählen und bewegen. Jede Einheit startet mit Gesundheit `1.0`. Der Client
sendet nur Zielpositionen; Bewegung und Terrainhöhe berechnet der Server.

Über **Leave game** unter **Auswahl** kannst du das Spiel verlassen.

Linksklick wählt eine eigene Einheit aus, Cmd + Linksklick fügt Einheiten zur Auswahl
hinzu oder entfernt sie. Ein Linksklick auf freie Fläche leert die Auswahl.
Halte die linke Maustaste gedrückt und ziehe einen Rahmen, um beim Loslassen
alle eigenen Einheiten darin auszuwählen. Cmd + Ziehen ergänzt die bestehende
Auswahl. Escape bricht die Rechteckauswahl ab.
Rechtsklick auf den Boden bewegt die ausgewählten Einheiten. WASD verschiebt
die Kamera, Q/E dreht sie, das Mausrad zoomt, die mittlere Maustaste verschiebt
die Kamera per Ziehen.

## Browser-Version

Voraussetzungen: Node.js (20 oder neuer), npm, SpacetimeDB CLI 1.1.2 und ein
erreichbarer SpacetimeDB-Server. Das Build-Skript generiert die Rust- und
TypeScript-Bindings automatisch aus dem Server-Modul.

```sh
bash game/build-web.sh
python3 -m http.server 8000 --directory public
```

Öffne http://localhost:8000. Das Build-Skript installiert bei Bedarf das
WebAssembly-Ziel und die zu `Cargo.lock` passende wasm-bindgen-CLI. Es erzeugt
eine statische Website in `public`. Die relativen Dateipfade funktionieren auch
unter einem Projektpfad wie `/rts/`. Ein Browser mit WebGL 2 wird benötigt.

## GitHub Pages

Der Workflow `.github/workflows/pages.yml` läuft bei jedem Push und kann unter
**Actions → Build and publish game → Run workflow** manuell gestartet werden.
Er prüft Formatierung, führt Tests aus und baut die Browser-Version.
Erfolgreiche Builds des Standardbranchs werden automatisch veröffentlicht.

Aktiviere im Repository unter **Settings → Pages → Build and deployment →
Source** die Option **GitHub Actions**, und pushe anschließend die Änderungen.
Die Spieladresse für dieses Repository lautet:
https://niki123456123456.github.io/rts/.

Der erste Build dauert wegen der Bevy-Kompilierung länger. Folgende Builds
verwenden den Cargo-Cache. Die extrahierten Stronghold-Assets im Stammverzeichnis
werden nicht veröffentlicht; die aktuelle Szene erzeugt ihre Geometrie selbst.
