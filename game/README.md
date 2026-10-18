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
Rechtsklick auf den Boden bewegt die ausgewählten Einheiten. Rechtsziehen legt
eine Reihenformation fest: Die Ziehrichtung bestimmt die Ausrichtung, die Länge
bestimmt die Breite und damit die Anzahl der Reihen. Die Formation ist um die
Mitte der gezogenen Linie zentriert; eine unvollständige letzte Reihe ist mittig.
Kleine Kreise zeigen während des Ziehens die Ziele aller ausgewählten Einheiten.
Loslassen erteilt den Befehl; Escape bricht ab. Blockierte Formationen werden
vom Server abgelehnt. WASD verschiebt
die Kamera, Q/E dreht sie, das Mausrad zoomt, die mittlere Maustaste verschiebt
die Kamera per Ziehen.

## Gebäude

Unter **Auswahl** starten **Kornspeicher**, **Steinkaserne** und **Vorratslager** den jeweiligen Baumodus.
Die Steinkaserne verwendet `military/barrack_stone/mesh.glb` mit `collision.glb` im selben Ordner; Einheitenrekrutierung ist noch nicht implementiert. Linksklick platziert
auf grün markiertem, ebenem Boden. Rot bedeutet, dass die Fläche blockiert ist oder
über den Kartenrand hinausgeht. Rechtsklick oder Escape bricht den Baumodus ab.
Gebäude dürfen weder andere Gebäude noch Einheiten überdecken. Die Vorschau
verwendet dieselbe Kollisionsfläche wie die serverseitige Wegfindung.

Linksklick auf ein Gebäude wählt es aus. Eigene Gebäude lassen sich mit
**Remove building** unter **Auswahl** entfernen. Rechtsklick auf ein feindliches
Gebäude befiehlt den ausgewählten eigenen Einheiten den Angriff. Sie verteilen
sich auf freie Positionen um das Gebäude und verursachen
erst an ihrer Angriffsposition in Reichweite Schaden. Ist die erste Reihe voll,
warten weitere Einheiten mit Abstand dahinter. Beim Zeigen auf ein feindliches
Gebäude erscheint mit ausgewählten eigenen Einheiten ein Symbol mit gekreuzten Schwertern.
Alle Gebäudetypen starten mit Gesundheit `1.0` und verschwinden bei
`0.0`. Die Anzeige unter **Auswahl** zeigt die aktuelle Gesundheit.

Der Server berechnet kürzeste Wege um Gebäude mit Abstand für die Einheiten.
Einheiten halten auch zueinander Abstand und können nicht durch andere Einheiten
laufen. Belegte Bewegungsziele werden auf freie benachbarte Positionen verteilt.
Neue oder entfernte Gebäude aktualisieren laufende Routen. Blockierte oder nicht
erreichbare Bewegungsziele werden abgelehnt. Pro Spiel sind bis zu 128 Gebäude möglich.
Eigene Gebäude werden beim Verlassen, Spielwechsel oder Verbindungsabbruch entfernt.
Die Gebäude-GLBs einschließlich eingebetteter Texturen liegen versioniert unter
`game/assets/models/buildings/supply/granary/` (Kornspeicher),
`supply/stockpile/` (Vorratslager) und `military/barrack_stone/` (Steinkaserne),
jeweils mit `mesh.glb` und `collision.glb`.
`housing/` ist für spätere Wohngebäude vorbereitet. Die Browser-Pipeline kopiert diese
Struktur nach `public/assets/`. `python3 game/import-granary.py` übernimmt die
Modelle und Kollisionsmodelle erneut aus dem ursprünglichen Asset-Archiv.
Dabei werden die Materialeinstellungen der Gebäude auf deckend gestellt,
da die importierten DDS-Alpha-Kanäle sonst Dach und Wände durchsichtig machen.
Beim Verteilen einer nativen Binärdatei muss der Ordner `assets` neben ihr liegen
(alternativ `BEVY_ASSET_ROOT` auf das Verzeichnis mit dem Asset-Ordner setzen).

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
werden nicht vollständig veröffentlicht; nur die benötigten Gebäudemodelle
aus `game/assets` werden mitgeliefert.
