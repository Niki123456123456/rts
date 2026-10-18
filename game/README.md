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
Jeder Spieler startet mit einem Burgherrn und kann nur seine eigenen
Kampfeinheiten auswählen und bewegen. Farbige Bodenringe zeigen die Zugehörigkeit. Jede Einheit startet mit Gesundheit `1.0`. Der Client
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
`fortifications/` enthält 19 Befestigungsmodelle: vier Torhäuser, Ausfallpforte,
Wachposten, sieben Turmtypen, Turmflagge, Wehrgang, drei Brückenelemente und die Steinmauer.
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

### Steinmauern

Unter **Befestigungen → Steinmauer** zuerst die Startzelle, dann die Endzelle
anklicken. Während der Auswahl zeigt eine grüne beziehungsweise rote Vorschau die
vollständige Linie. Nach dem Platzieren können weitere Linien gebaut werden;
Rechtsklick oder Esc beendet den Modus. Schräge Linien bilden lückenlose Rasterstufen.
Vorhandene eigene Mauersegmente werden beim Verlängern wiederverwendet. Die gesamte
Linie wird atomar geprüft: Ein belegtes Feld verhindert den kompletten Auftrag.

Jedes Segment belegt eine 2×2-m-Zelle und hat eine 6,663 m hohe Mauerkrone, exakt auf
Höhe des Treppenturmdachs. Ein Segment bietet neun reservierbare Plätze; ein Rechtsklick
mit ausgewählten Einheiten verteilt diese auf die nächsten freien Plätze des gesamten
zusammenhängenden Mauerabschnitts. Kleine Kreise zeigen die Zielpositionen vorab.
Nur seitlich verbundene eigene Segmente auf gleicher Geländehöhe bilden einen Weg.
Eckkontakt zählt nicht als Verbindung.

Ein Abschnitt muss an einen eigenen Treppen-, Aussichts- oder viereckigen Turm
anschließen. Einheiten gehen zur erreichbaren Turmtür, werden durch die Luke versetzt
und laufen über die Mauerkrone. Höhere Türme haben einen internen Übergang zur
Mauerebene. Der Rückweg führt über eine freie Tür eines angeschlossenen Turms.
Ohne Turmverbindung sind keine neuen Mauerbefehle oder Abstiege möglich. Bei Abriss
werden betroffene Einheiten auf freien Boden evakuiert und betroffene Laufwege beendet.
Alle Mauersegmente blockieren die Bodennavigation, unabhängig von ihrer Begehbarkeit.
Pro Spiel sind zusätzlich zu den 128 Gebäuden bis zu 4096 Mauersegmente möglich.

`game/create-stone-wall.py` erzeugt `fortifications/stone_wall/{mesh,collision}.glb`
und wird auch vom Modellimport aufgerufen. Eingebettet werden die vorhandenen
Texturen `assets/images/Source/meshes/buildings/castle_wall.png` (Seiten) und
`walltop.png` (Mauerkrone). Bestehende Turmmodelle werden dabei nicht verändert.

### Zivile Versorgungsgebäude

Die Kategorie **Versorgung** enthält 37 Einträge. Neben Kornspeicher und Vorratslager
sind alle zivilen Versorgungsketten als platzierbare Modelle importiert:

- Nahrung: Obstgarten, Käserei, Getreidefarm, Hopfenfarm, Weinberg, Schweinezucht,
  Schafzucht, Jägerstand, Fischteich, Taubenschlag, Gärtnerhütte, Gemüsebeet und Bienenstock.
- Verarbeitung und Vorräte: Mühle, Bäckerei, Brauerei, Weingut, Burgküche,
  Vorratskammer der Diener, Schenke und Markt.
- Rohstoffe und Transport: Holzfällerhütte, Steinbruch, Eisenmine, Pechgrube,
  Ochsenjoch und Fuhrmannsposten.
- Weitere Versorgung: Weberei, Kerzengießerei, Brunnen, Wassertrog, Jauchegrube,
  Apotheke, Falknerposten und Schatzkammer.

Jedes Gebäude liegt in einem englisch benannten Unterordner von
`game/assets/models/buildings/supply/` mit `mesh.glb` und `collision.glb`.
Der Import verwendet die vollständigen Basismodelle, bevorzugt die geschlossene
Variante, und vorhandene Kollisionsmodelle. Für Fischteich, Taubenschlag,
Gemüsebeet und Wassertrog wird eine konservative Box erzeugt. Der gemeinsame
Katalog liefert Rastergröße, Rotation, Bauvorschau, Auswahl und Bodennavigation.
Die Modelle sind baubar. Steinbruch, Eisenmine und Holzfällerhütte produzieren bereits
Rohstoffe; weitere Produktionsketten sind noch nicht implementiert.

Beim Start lädt eine begrenzte Warteschlange jeweils nur das Modell- und
Kollisionspaar eines Gebäudes. Ein im Baumenü gewähltes, noch nicht angefordertes
Gebäude wird als Nächstes geladen. Das verhindert, dass sich die glTF-Texturaufgaben
bei vielen gleichzeitigen Importen auf dem IO-Worker verschachteln und dessen
Stack im nativen Debug-Build überlaufen. Die vollständige Startwarteschlange wird
im Test `startup_loads_full_catalog_with_only_one_building_pair_in_flight` geprüft.

### Ressourcen und Baukosten

Unter **Versorgung** lässt sich die **Unterkunft** bauen (Symbol: Zeile 6,
Spalte 5). Sie verwendet `hovel_small.glb` mit dem zugehörigen Kollisionsmodell,
liegt unter `assets/models/buildings/supply/housing/` und belegt 3×3 Rasterfelder
(6×6 m). Jede Unterkunft bietet vier Bauern und benötigt selbst keinen Bauern.
Die im alten Mesh fehlenden Materialien werden beim Import eingebettet:
`stone_mossy.png` für die Wände (Ersatz für die nicht enthaltene
`hovel_small.bmp`) und `hovel_small_roof_2.alphatest.png` für das Strohdach.
Nur Gebäude mit tatsächlicher Produktion (Steinbruch, Eisenmine und
Holzfällerhütte) belegen je einen Bauern. Befestigungen, Vorratslager,
Kornspeicher und Kaserne benötigen keine Bauern. Oben rechts steht beim
Bauernsymbol **belegt/Kapazität**, etwa **4/8**. Man beginnt bei **0/0**; eine Unterkunft ist erst für Produktionsgebäude nötig.
Abriss oder Zerstörung gibt den belegten Bauern frei; bei Unterkünften sinkt die
Kapazität. Bestehende Betriebe bleiben bei Überbelegung erhalten, weitere
Produktionsgebäude benötigen wieder freie Bauern. Die Begrenzung gilt pro Spieler
und wird vom Server geprüft. Mauern lassen sich auch ohne freie Bauern bauen.
Kampfeinheiten werden weiterhin unabhängig davon in der Kaserne ausgebildet.

Integrationstest: `npm --prefix game/web run test:housing` gegen eine Testdatenbank.

Jeder neue Spielbeitritt beginnt **ohne Lager und mit 0 Holz, 0 Stein und 0 Eisen**.
Beim Bau des ersten eigenen Vorratslagers gibt es einmalig **20 Holz, 20 Stein und
20 Eisen** direkt in dieses Lager. Weitere Lager und Wiederaufbauten gewähren
keinen zusätzlichen Bonus. Das erste Lager ist kostenlos.
Oben rechts zeigt die Ressourcenleiste den vom Server bestätigten eigenen Bestand.
Die Symbole stammen aus `assets/images/Source/ui/ingameui.png`, Reihe 9 ab Spalte 6
(1-basiert), und werden aus dem unveränderten Atlas unter `game/assets/ui/` angezeigt.
Während die erste Bestandsmeldung geladen wird, erscheint eine Auslassung statt
eines geratenen Werts.

Ein Steinmauersegment kostet **1 Stein**, jeder der sieben eigentlichen Turmtypen
kostet **5 Stein**. Eine Mauerlinie berechnet nur neu gebaute Segmente. Bei belegten
Baufeldern oder fehlenden Ressourcen wird der vollständige Bauauftrag einschließlich
aller Abbuchungen zurückgerollt. Die Vorschau wird bei unzureichendem Bestand rot;
Werkzeughinweise und Bauleiste zeigen die Steinpreise an. Andere Gebäude sind
vorerst kostenlos. Holz und Eisen werden noch nicht zum Bauen verbraucht.
Abriss erstattet keine Ressourcen.

Bestände gelten pro Spieler und Spielteilnahme. Ein wiederholter Beitritt zum bereits
aktiven Spiel setzt sie nicht zurück; beim Verlassen werden sie mit der Teilnahme
entfernt. Ein neuer Beitritt beginnt wieder ohne Lager und Bestand; die Gabe gibt es erst
beim Bau des ersten Lagers dieser neuen Teilnahme.

### Physisches Lager und Produktion

Das erste Vorratslager wird vom Spieler selbst platziert. Der geschenkte Bestand
liegt darin als sichtbare Holz-, Stein- und Eisenmodelle auf einem inneren Raster
mit 6×4 Plätzen im Abstand von 2 m. Teilmengen und volle Lagen stellen die Anzahl
exakt dar. Pro Platz passen 20 Holz, 54 Steine oder 96 Eisen; Ressourcen teilen
sich keinen Platz. Das Lager verwendet das offene Originalmodell, damit der Inhalt
sichtbar bleibt. Zusätzliche Vorratslager starten leer. Baukosten entnehmen Stein
aus den eigenen Lagern. Wird ein Lager zerstört oder abgerissen, geht dessen Inhalt verloren.

**Holzfällerhütte → Holz**, **Steinbruch → Stein**, **Eisenmine → Eisen**: Jeder Betrieb
erzeugt eine Einheit alle 3 Sekunden und puffert höchstens 20 Einheiten. Beim Bau
entsteht automatisch ein brauner Transportbauer mit Hut. Er lässt sich nicht auswählen
oder befehligen und trägt bis zu 5 Einheiten pro Weg. Bauern verwenden dieselben
serverseitigen Wege, Hindernisse und Abstände wie andere Bodeneinheiten. Sie laufen
vom Betrieb zu einem erreichbaren eigenen Lager und zurück. Ohne Platz oder
erreichbares Lager warten sie. Erst die tatsächliche Ablieferung erhöht den
verfügbaren Bestand. Ausgewählte Betriebe zeigen Puffer und unterwegs befindliche
Ladung; ausgewählte Lager zeigen Inhalt und belegte Plätze.

Die 22 Ressourcenmodelle liegen unter `game/assets/models/resources/` und werden
mit `python3 game/import-storage.py` aus den Original-Stockpile-Modellen importiert.
Auch diese Modelle werden nacheinander geladen.

### Einheitentypen und Rekrutierung

Neue Teilnahmen starten mit genau einem **Burgherrn** (`lord.glb`). Wähle eine
eigene **Steinkaserne** aus und klicke auf das Bauern- oder Bogenschützensymbol,
um **bewaffnete Bauern** (`armed_peasant.glb`) oder **Bogenschützen** (`archer.glb`)
zu rekrutieren. Die neue Einheit erscheint sofort
auf einer freien Position vor der Kaserne; deren Rotation dreht den Aufstellbereich
mit. Ohne freien Platz schlägt die Rekrutierung fehl. Sie ist derzeit kostenlos;
bis zu 200 Kampfeinheiten pro Spieler sind möglich. Transportbauern bleiben
automatisch gesteuert und zählen nicht zu diesem Limit.

Die Typen verwenden ihre Original-Skelettmodelle: Der Burgherr trägt
`lord_sword.glb`, der bewaffnete Bauer `armed_peasant_fork.glb`. Die Waffen sind
am rechten Handknochen befestigt. Bogenschützen tragen `archer_bow.glb` an der
linken Hand; die Schussanimation zieht die rechte Hand zur Wange. Da die gelieferten Modelle keine passenden
Clips enthalten, erzeugt `python3 game/import-units.py` eigene Skelettanimationen
für `idle`, `walking` und `fighting` in `game/assets/models/units/`. Die Clips
bewegen Arme, Beine und Oberkörper; Waffen folgen der Hand. Animationen laufen
am Ort, Positionsänderungen berechnet weiterhin ausschließlich der Server.

Der Server repliziert Einheitentyp, Tätigkeit und Blickrichtung. Bewegung startet
die Laufanimation, Stillstand die Standanimation und ein tatsächlich ausgeführter
Angriff in Reichweite die Kampfanimation. Übergänge werden über 140 ms überblendet.
Bestehende Armeen bleiben bei einem Serverupdate erhalten und werden einmalig
typisiert; für den neuen Start mit einem Burgherrn eine neue Teilnahme beginnen.

### Kampf gegen Einheiten

Wähle eigene Kampfeinheiten und klicke mit rechts auf eine gegnerische Einheit.
Zusammenstehende Gegner gehören zu einer Gruppe, wenn sie über Nachbarn mit
höchstens 3 m Abstand verbunden sind; verschiedene Höhen werden berücksichtigt.
Rote Ringe und die Gegneranzahl zeigen vor dem Klick, welche Gruppe gemeint ist.
Angreifer verteilen sich auf freie Nahkampfplätze und wechseln nach dem Tod eines
Gegners zum nächsten Mitglied der angeklickten Gruppe. Entfernte Gegner gehören
nicht automatisch zu diesem Auftrag.

Im Nahkampf müssen Gegner höchstens 1,05 m entfernt und auf gleicher Höhe sein;
Mauern und Gebäude verhindern Treffer hindurch. Die Einheiten benutzen die normale
Wegfindung und halten Abstand. Beide Seiten kämpfen mit ihren Animationen und
verlieren Gesundheit. Lebensbalken erscheinen bei Kampf oder Verletzungen. Bei
null Gesundheit wird die Einheit entfernt und gibt ihre Position und Dachplätze frei.
Der Burgherr verursacht 0,22 Schaden/s, bewaffnete Bauern 0,14 Schaden/s.

Ohne laufenden Bewegungsbefehl greifen Kampfeinheiten erreichbare Gegner innerhalb
von 8 m selbständig an. Automatische Verfolgung reicht höchstens 16 m vom
Ausgangspunkt; nach dem Gefecht oder einem entkommenen Gegner laufen sie dorthin
zurück. Ist die Stelle inzwischen belegt, wählen sie einen freien Platz daneben.
Ein neuer Bewegungs-, Dach- oder Angriffsbefehl ersetzt den bisherigen Auftrag.
Manuell befohlene Gruppenangriffe enden nach dem letzten erreichbaren Gegner;
die überlebenden Angreifer bleiben am Kampfplatz. Transportbauern nehmen nicht
an diesen Kämpfen teil. Gegen Besatzungen auf Mauern und Türmen laufen Angreifer
zu einer erreichbaren Turmtür und gelangen über die Luke auf das verbundene Dach-
und Mauernetz – auch bei feindlichen Befestigungen. Freie Angriffsplätze werden
auf dem gesamten verbundenen Netz reserviert, sodass ein volles Mauerstück über
Nachbarstücke angegriffen werden kann. Nahkampf funktioniert auch über die Grenze
zweier Mauerstücke. Verteidiger greifen zurück an und kehren danach auf ihren
ursprünglichen Dachplatz zurück. Fehlender Zugang oder voller Platz wird in der
Vorschau rot angezeigt; es gibt keine Treffer vom Boden durch die Mauer hindurch.

### Bogenschützen

Bogenschützen verwenden ausschließlich Pfeile, auch aus nächster Nähe. Auf einen
Angriffsbefehl laufen sie bis zur nächsten freien Schussposition und bleiben stehen,
sobald der Gegner in Reichweite und die Schussbahn frei ist. Bei automatischer
Verteidigung bleiben sie an ihrer Position und verfolgen keine Gegner.
Die Grundreichweite beträgt 14 m; jeder Meter Höhe über der Kartenebene bringt
0,6 m hinzu, auch auf Hügeln. Ausgewählte Bogenschützen zeigen ihre maximale
Reichweite als hellblauen Kreis. Gelände und Gebäude können die Pfeilbahn blockieren.

Alle 1,25 s wird ein sichtbarer Pfeil (`archer_arrow.glb`) abgeschossen. Er fliegt
mit 22 m/s auf einer Bogenbahn zum Ziel und verursacht beim Einschlag 0,18 Schaden
an einer Einheit bzw. 0,06 an einem Gebäude. Der Server berechnet Schüsse und Treffer;
Browser und native Anwendung stellen dieselben replizierten Projektile dar.
