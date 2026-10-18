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

Die Spielfläche ist 800×800 Welteinheiten groß (vorher 80×80). Einheiten bewegen sich innerhalb
±399; Bauflächen dürfen bis an die Kartengrenzen bei ±400 reichen. Die Kamera
kann das gesamte Gebiet erreichen.
Die ursprünglichen Zoom-Einstellungen bleiben erhalten.

## Gebäude

Unter **Auswahl** wählt die Kategorienliste zwischen **Versorgung**, **Militär** und
**Befestigungen**. Die Gebäudeliste lässt sich horizontal scrollen. Ein Gebäudeknopf
startet den jeweiligen Baumodus.
Die Steinkaserne verwendet `military/barrack_stone/mesh.glb` mit `collision.glb` im selben Ordner; Einheitenrekrutierung ist noch nicht implementiert. Linksklick platziert
auf grün markiertem, ebenem Boden. Rot bedeutet, dass die Fläche blockiert ist oder
über den Kartenrand hinausgeht. Rechtsklick oder Escape bricht den Baumodus ab.
Mit **R** dreht sich das Gebäude im Baumodus um jeweils 90°. Vorschau und Baufläche
drehen sich gemeinsam; bei 90° und 270° tauschen Rasterbreite und Rastertiefe.
Die Ausrichtung wird auf dem Server gespeichert und bei allen Spielern angezeigt.
Beim eigenen **Treppenturm**, **Aussichtsturm** und **viereckigen Turm** können
ausgewählte Einheiten per **Rechtsklick aufs Dach** nach oben geschickt werden. Grüne Kreise zeigen die vorgesehenen Dachplätze.
Der Abstand beträgt auf allen Dächern **0,70 m**. Treppenturm und Aussichtsturm
bieten **5×5 Plätze (25 Einheiten)**, der viereckige Turm **8×8 Plätze (64 Einheiten)**.
Die Anzahl der Reihen und Spalten ergibt sich aus der nutzbaren Dachfläche mit
Abstand zur Dachkante. Überzählige Einheiten behalten ihre
bisherigen Befehle. Einheiten laufen zu einer erreichbaren Tür und werden direkt
zur Luke teleportiert. Es gibt keine Warteschlange oder Begrenzung gleichzeitig
teleportierter Einheiten. Von der Luke verteilen sie sich auf ihre Dachplätze;
ankommende Einheiten können dabei aneinander vorbeilaufen. Boden- und Angriffsbefehle
führen zuerst zur Luke und teleportieren die Einheiten zurück an die Tür.
Auch ein direkter Befehl auf ein anderes eigenes Turmdach nutzt diesen Weg.
Gedrehte Türme verwenden entsprechend gedrehte Türen, Luke und Dachplätze.
Dachhöhe, Dachgrenzen und Zugänge werden je Modell festgelegt. Der
Aussichtsturm hat zwei Türen, der viereckige Turm eine. Die ursprünglichen Meshes
bleiben erhalten.
Das unsichtbare Bauraster besteht aus 2×2-m-Feldern. Die Vorschau rastet so ein,
dass alle Kanten der logischen Baufläche auf Feldgrenzen liegen. Gebäude dürfen
belegte Rasterfelder nicht überdecken; gemeinsame Außenkanten sind erlaubt.
Die Bauprüfung und die angezeigte Fläche verwenden die Rastergrößen aus dem
Gebäudekatalog. Kollisionsmodelle bestimmen weiterhin Auswahl, Wegfindung und
Angriffspositionen. Einheiten blockieren den Bau in der Baufläche und im Bereich
der tatsächlichen Gebäudegeometrie, damit sie nicht eingeschlossen werden.

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
`fortifications/` enthält 18 Befestigungsmodelle: vier Torhäuser, Ausfallpforte,
Wachposten, sieben Turmtypen, Turmflagge, Wehrgang und drei Brückenelemente.
`housing/` ist für spätere Wohngebäude vorbereitet. Die Browser-Pipeline kopiert diese
Struktur nach `public/assets/`. `python3 game/import-buildings.py` übernimmt die
Modelle und Kollisionsmodelle erneut aus dem ursprünglichen Asset-Archiv.
`game/building-models.json` ist der Importkatalog; das Skript erzeugt auch
`shared/src/building_catalog.rs` mit Modellpfaden und Kollisionsmaßen.
Fehlt ein Kollisionsmodell, wird ein umschließender Quader erzeugt. Torhäuser und
Brücken werden derzeit wie die anderen Gebäude als feste Hindernisse behandelt;
Torsteuerung und begehbare Brückendecks sind noch nicht implementiert.
Dabei werden die Materialeinstellungen der Gebäude auf deckend gestellt,
da die importierten DDS-Alpha-Kanäle sonst Dach und Wände durchsichtig machen.
Beim Verteilen einer nativen Binärdatei muss der Ordner `assets` neben ihr liegen
(alternativ `BEVY_ASSET_ROOT` auf das Verzeichnis mit dem Asset-Ordner setzen).

### Logische Bauflächen

Die Rastergröße steht pro Gebäude als `grid: [Breite, Tiefe]` in
`game/building-models.json`. Ein Treppenturm belegt 2×2 Felder (4×4 m);
zwei direkt benachbarte Treppentürme belegen gemeinsam 4×2 Felder (8×4 m).
Die Modelle behalten ihre ursprüngliche Größe; kleine Überstände dürfen sich
bei benachbarten Bauflächen berühren oder überlappen. Bereits gebaute Gebäude
werden bei einem Serverupdate nicht verschoben.

| Gebäude | Felder | Meter |
|---|---|---|
| Kornspeicher | 3×3 | 6×6 |
| Steinkaserne | 6×5 | 12×10 |
| Vorratslager | 7×5 | 14×10 |
| Hölzernes Torhaus | 2×1 | 4×2 |
| Kleines Torhaus | 3×3 | 6×6 |
| Mittleres Torhaus | 7×6 | 14×12 |
| Großes Torhaus | 9×8 | 18×16 |
| Ausfallpforte | 3×3 | 6×6 |
| Wachposten | 1×1 | 2×2 |
| Holzturm | 1×1 | 2×2 |
| Aussichtsturm | 2×2 | 4×4 |
| Treppenturm | 2×2 | 4×4 |
| Viereckiger Turm | 3×3 | 6×6 |
| Niedriger Rundturm | 5×5 | 10×10 |
| Hoher Rundturm | 5×5 | 10×10 |
| Großer Wehrturm | 7×7 | 14×14 |
| Turmflagge | 1×1 | 2×2 |
| Wehrgang | 1×1 | 2×2 |
| Steinbrücke: Ende | 2×2 | 4×4 |
| Steinbrücke: Mitte | 2×2 | 4×4 |
| Holzbrücke | 2×23 | 4×46 |

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
