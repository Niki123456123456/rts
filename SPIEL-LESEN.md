# Stronghold mit Landschaften und Minimap

Auf dem bisherigen Testrechner zum Spielen SPIEL-STARTEN.cmd doppelklicken. Das Skript startet die gespeicherte
lokale Spielwelt und den Webserver und öffnet http://127.0.0.1:18765/.
Nach einem Neustart das Startskript erneut ausführen. Die neue lokale Version
benötigt zum Spielen keine Internetverbindung. Python ist bereits installiert;
der Datenbankserver ist dort im Ordner runtime installiert. Beim Download von
GitHub bitte zuerst ZUSAMMENARBEIT.md lesen: Runtime und lokale Datenbank
werden nicht ins Repository hochgeladen.

## Landschaft

Die 800 × 800 Meter große Karte enthält offenes Grasland, zwei Waldgebiete mit
Bäumen, Wüsten mit kleinen Dünen, Bergketten und sieben Eisenvorkommen.
Zusätzliche Stein- und Eisenplätze, ein weiteres Waldstück, eine Sandfläche und
kleinere Berge liegen nahe den Startburgen, sodass mehrere Geländetypen früh
erreichbar sind. Ein Fluss
durchzieht die Karte. Drei trockene Furten verbinden die Ufer. Einheiten umgehen
Flussabschnitte und Bergspitzen; die Bewegungshöhe entspricht der sichtbaren
Geländeoberfläche. Holzfällerhütten brauchen Wald, Steinbrüche ein Steinvorkommen
und Eisenminen ein Eisenvorkommen. Normale Gebäude dürfen auf Gras, Sand und im
Wald stehen; Fels, Erzfelder und Wasser sind dafür gesperrt. Auf hügeligem Sand
gleicht ein Fundament die Bauhöhe aus. Bäume auf der Baufläche werden automatisch
entfernt. Die Rodung wird pro Spielwelt gespeichert und bleibt auch nach Abriss
und erneutem Verbinden erhalten. Fehlgeschlagene Bauversuche entfernen keine Bäume.

## Palast und Zuwanderer

Jeder Spieler erhält beim Erstellen oder Betreten eines Spiels automatisch
genau einen Palast. Die Kamera zeigt beim Start die eigene Burg samt Vorplatz.
Die Startplätze der Spieler überlappen sich nicht. Erneutes Beitreten während
derselben Teilnahme erzeugt keine zweite Burg. Der Palast ist das einmalige Startgebäude und kann nicht über das Baumenü gebaut
werden. Nach Abriss wird kein zweiter Palast vergeben. Der runde Bergfried ist
ein eigenes, baubares Verteidigungsgebäude unter „Befestigungen“.
Er hat einzelne Steine und Dachziegel als 3D-Geometrie, eine begehbare Terrasse,
eine Außentreppe und eine Feuerstelle mit Bänken. Der vorhandene Burgherr nimmt
beim Start seinen Platz auf der Terrasse ein. Den Burgherrn auswählen und mit
rechts auf freien Boden klicken: Er läuft über die Außentreppe hinunter.
Mit Rechtsklick auf die eigene Terrasse kann er wieder hinaufsteigen. Der Burgherr
ist direkt auf der Terrasse anklickbar. Bewaffnete Bauern und Bogenschützen
benutzen dieselbe Außentreppe; auch auf dem Treppenabsatz kann ein neuer
Befehl zum Boden erteilt werden.

Der Palast bietet vier Wohnplätze, jede Unterkunft vier weitere. Freie Bauern
warten sichtbar am Lagerfeuer. Produktionsgebäude können zunächst leer gebaut
werden. Freie Bauern werden automatisch offenen Betrieben zugeteilt und laufen
vom Lagerfeuer zum Arbeitsplatz. Ein Produktionsgebäude anklicken und „Gebäude
schließen“ wählen, um die Produktion anzuhalten und den Bauern freizugeben.
Geschlossene Betriebe bleiben auch bei weiterem Zuzug unbesetzt. „Gebäude öffnen“
gibt den Arbeitsplatz wieder für die automatische Vergabe frei. Sind gerade keine
Bauern frei, wartet der Betrieb auf den nächsten Zuwanderer. Bauern umgehen die
Gebäudeflächen auf ihren Arbeitswegen; auch die Ankunft am Feuer nutzt sichere Wege.
Ohne angekommenen Bauern am Arbeitsplatz gibt es keine Produktion.

Das neue Gesicht-Symbol oben zeigt die Zufriedenheit. Ein Klick auf das Gesicht oder den Prozentwert öffnet die Übersicht mit allen
Einflussfaktoren und den Bevölkerungsgrenzen für spätere Bedürfnisse. Die ersten
vier Bauern bleiben auch ohne Nahrung bei mindestens 80% zufrieden, solange
Wohnraum vorhanden ist. Ab fünf Bewohnern zählt fehlende Nahrung, ab acht Wasser,
ab 16 Nahrungsvielfalt und Arbeitsplätze, ab 24 Gesundheit, ab 32 Freizeit und
ab 40 Warenbestände. Die Grundzufriedenheit sinkt dabei von 75 auf 65, 55 und
schließlich 50 Punkte. Unter „Bedürfnisse“ sind Brunnen,
Apotheke, Wirtshaus und Garten verfügbar. Ein Kornspeicher liefert den ersten
Nahrungsvorrat. Alle 30 Sekunden wird je vier Bauern eine Ration verbraucht.

Bei 80–100 Zufriedenheit kommt alle vier Sekunden ein Bauer, bei 65–79 alle acht
und bei 50–64 alle vierzehn Sekunden. Bei 40–49 gibt es keinen Zuzug. Unter 40
wandert alle vierzehn Sekunden ein freier Bauer ab. Ohne freien Wohnraum ziehen
keine weiteren Bauern zu. Beschäftigte Bauern bleiben an ihrem Arbeitsplatz.
Bevölkerung, Vorräte, Zufriedenheit und Zuweisungen verwaltet der Spielserver.
Die Darstellung zeigt bis zu 16 Wartende gleichzeitig als Figuren.


## Rohstoffe, Karren und Wildtiere

Ein neuer Spieler beginnt mit 120 Holz, 120 Stein und 120 Eisen. Die Bauleiste
zeigt die Kosten für jedes Gebäude. Das erste Vorratslager übernimmt den noch
verfügbaren Startvorrat; spätere Lager erzeugen keine neuen Rohstoffe.

Bäume sind echte Holzbestände: Der Holzfäller fällt einen Baum, der sichtbar
umkippt, und zerlegt anschließend den Stamm. Stein- und Eisenvorkommen werden
beim Abbau kleiner und sind irgendwann erschöpft. Rodung beim Bauen liefert
keine zusätzlichen Rohstoffe.

Für Stein und Eisen ist ein Ochsenkarren erforderlich. Einen Ochsenplatz bauen,
den Steinbruch oder die Eisenmine auswählen und „Karren zuweisen“ anklicken.
Jeder Ochsenplatz stellt einen Karren für einen Betrieb bereit. Holzfäller können
optional ebenfalls einen Karren erhalten: Er transportiert 20 statt 5 Einheiten.
Ein beladener Karren muss erst entladen werden, bevor er neu zugeteilt wird.
Ohne passenden Lagerplatz wartet der Arbeiter mit seiner Ladung.

Fischschwärme liegen nur in bestimmten Flussabschnitten und erscheinen auf der
Minimap. Eine Fischerei braucht einen passenden Fischgrund in Ufernähe. Ihr
Fang verringert diesen Bestand; er erholt sich langsam. Fisch wird zum
Kornspeicher gebracht. Jägerhütten jagen Wild in ihrer Umgebung und liefern
Wildfleisch. Rehe, Hasen und Wildschweine leben in unterschiedlich großen Gruppen.
Wölfe und Bären verteidigen ihr Revier; gelegentlich ziehen sie in erreichbare,
offene Siedlungen und können dort Bauern angreifen. Ausgewählte Soldaten können
über einen Rechtsklick auf ein Tier ebenfalls einen Jagdauftrag erhalten.

## Mauern, Dächer und Tore

Gebäude können nicht in bestehende Gebäude oder Mauertreppen gebaut werden.
In „Befestigungen“ stehen niedrige, normale und hohe Mauern sowie Zinnenmauern
zur Auswahl. Start- und Endpunkt anklicken, um eine zusammenhängende Mauer zu
ziehen. Die Zinnen sind Hindernisse für die tatsächliche Flugbahn von Pfeilen.
Mauerflächen gleicher Höhe verbinden sich; unterschiedliche Höhen brauchen
geeignete Zugänge.

Für „Mauertreppe ziehen“ zuerst den freien Fußpunkt und danach die Oberseite
einer eigenen Mauer anklicken. Die Treppe kann von der Mauer weg oder mit etwas
seitlichem Abstand parallel zur Mauer verlaufen. Ihr oberer Absatz verbindet
sie mit dem Wehrgang. Höhere Mauern brauchen längere Treppen. Einheiten auswählen
und mit rechts auf einen erreichbaren Wehrgang oder ein begehbares Gebäudedach
klicken. Sie bleiben auch oben anklickbar und benutzen beim Abstieg den Zugang.

Kleine und große Stadttore lassen sich über ihr Gebäudefenster öffnen und
schließen. Das Tor mit Zugbrücke wird so ausgerichtet, dass seine Brücke vom
Tor aus zum anderen Ufer reicht; R dreht die Vorschau. „Tor öffnen / Brücke
senken“ macht den Übergang begehbar. Solange Einheiten im Durchgang stehen,
lässt sich das Tor nicht schließen. „Graben ziehen“ gräbt einen vier Meter
breiten Wassergraben zwischen zwei gewählten Punkten; eine Zugbrücke kann auch
diesen überqueren.

Der schmale Aussichtsturm ist der höchste Turm. Die Reichweite von Bogenschützen
steigt mit der tatsächlichen Standhöhe aus Gelände und Gebäude. Die angezeigte
Reichweite und der Server verwenden dieselbe Regel. Gebäude, Gelände und Zinnen
können Pfeile auf ihrer Flugbahn abfangen.

Soldaten brauchen die angezeigten Rohstoffe und jeweils einen freien Bauern.
Die Rekrutierung an der eigenen Kaserne entnimmt ihn dem Bauernpool. Sind alle
Bauern beschäftigt, einen Betrieb schließen oder auf weitere Zuwanderer warten.

## Minimap

Die Minimap sitzt ohne Titel bündig in der unteren rechten Bildschirmecke und
überlagert die Gebäudeleiste. Am Griff oben links lässt sie sich vergrößern oder
verkleinern. Klicken oder Ziehen auf der Kartenfläche versetzt die Kamera.
Der weiße Rahmen zeigt den Kameraausschnitt. Gelände erscheint in passenden Farben;
beim Darüberfahren wird der Geländetyp angezeigt. Einheiten und Gebäude erscheinen
in Spielerfarben.

## Spielwelt und Updates

Die Landschaft nutzt die eigene lokale Datenbank stronghold-local auf Port 18766.
Lokale Spielwelten bleiben im Ordner server/.spacetime-local gespeichert. Dieser
Ordner enthält auch lokale Anmeldedaten und darf nicht öffentlich geteilt werden.
Die bisherigen Maincloud-Spielstände wurden nicht verändert. Die GitHub-Pages-Seite
zeigt weiterhin die ältere Version und wurde nicht aktualisiert.

Für spätere Quellcodeänderungen ist Build-Spiel.ps1 vorhanden; bei Änderungen
am Server oder am geteilten Gelände zusätzlich -ServerChanged übergeben.
