# Gemeinsam am RTS arbeiten

Die Version enthält die Minimap, Gelände und Bauregeln, einen automatischen
Bergfried pro Spieler sowie Zufriedenheit, Zuwanderung und manuell zugewiesene Bauern.

Vor Änderungen `git pull --rebase` bei sauberem Arbeitsverzeichnis ausführen.
Nach dem Testen Änderungen committen und `git push` ausführen. Falls der Push
abgewiesen wird, zuerst neue Beiträge abrufen, zusammenführen und erneut testen.
Keine gemeinsamen Commits mit einem Force-Push überschreiben.
Git synchronisiert gespeicherte Beiträge; beide Rechner bearbeiten ihre eigene Kopie.

## Auf einem zweiten Rechner spielen

Benötigt werden Rust (siehe rust-toolchain.toml), Node.js 22+, SpacetimeDB 1.1.2
und die in game/README.md beschriebenen nativen Bibliotheken. Befehle in Bash/WSL
aus dem Repository-Hauptordner ausführen.

1. `bash server/start.sh` in einem Terminal laufen lassen.
2. In einem zweiten Terminal:

```sh
export SPACETIMEDB_URI=http://127.0.0.1:3000
export SPACETIMEDB_DATABASE=stronghold-terrain
spacetime login --server-issued-login "$SPACETIMEDB_URI"
bash server/publish.sh
bash game/build-web.sh
python3 -m http.server 18765 --bind 127.0.0.1 --directory public
```

Danach http://127.0.0.1:18765/ öffnen. Für gemeinsame Multiplayer-Partien müssen
beide Clients denselben erreichbaren Spielserver verwenden. GitHub synchronisiert
den Programmcode, nicht die lokalen Spielstände oder den laufenden Spielserver.

Die Windows-Startdateien stammen vom bisherigen Testrechner und setzen die dort
separat installierten Runtime-Dateien und lokale Datenbank voraus. Diese privaten
Dateien werden nicht auf GitHub hochgeladen.

## Öffentliche Spielseite

Der bisherige Maincloud-Server hat das neue Datenbankschema noch nicht. Deshalb
wird die Pages-Seite erst nach expliziter Konfiguration des passenden Servers
aktualisiert: Den neuen Server bereitstellen und die Repository-Variablen
SPACETIMEDB_URI (HTTPS-Endpunkt) und SPACETIMEDB_DATABASE setzen. Anschließend
den Build-and-publish-Workflow starten. CI baut und prüft den Quellcode auch ohne
diese Variablen, überspringt dann aber die öffentliche Veröffentlichung.

Die lokale Testversion ist bereits gebaut und die Änderungen wurden mit 30
Shared-Tests, 10 Server-Tests und echten Server-Integrationstests für Gelände,
Bergfried, Zufriedenheit und Bauernzuweisung geprüft.
