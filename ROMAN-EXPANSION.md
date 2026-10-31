# Römische Erweiterung

Die Bauauswahl hat acht Bereiche in zwei Spalten. Namen, Kosten und Steuerung
stehen am Symbol; nach Auswahl erscheinen ausführliche Gebäudeinformationen.

- **Militär:** Castra, Waffenschmied, Waffenlager, Söldnerzelt und Fabrica.
  Der Schmied produziert Bögen, Schwerter oder Schilde. Eingelagerte Waffen
  ersetzen den entsprechenden Rohstoffanteil der Rekrutierungskosten.
  Söldner kosten Sesterzen und benötigen keine freien Bauern.
- **Versorgung:** Das Valetudinarium heilt nahe eigene Einheiten außerhalb des
  Nahkampfs. Thermen und Tempel verbessern die Versorgung und Zufriedenheit.
- **Wirtschaft:** Werkzeuge beschleunigen die Arbeit und verschleißen.
  Weinberge liefern Trauben an die Kelter; Olivenhaine liefern Oliven an die
  Ölpresse. Die fertigen Güter werden eingelagert und können verkauft werden.
  Wein und Öl bringen jeweils +2 Zufriedenheit; ab 16 Bewohnern wird alle
  30 Sekunden jeweils eine vorhandene Einheit verbraucht.
- **Forschung:** Drei Stufen für Waffen, Wirtschaft und Politiken. Ein Gelehrter
  arbeitet im Forschungszentrum. Eine Bibliothek beschleunigt Forschung.
- **Wunder:** Aquädukt, Amphitheater, Theater, Triumphstatue und Bibliothek.
  Zufriedenheitsboni gleicher Wunder werden nicht mehrfach angerechnet.

## Wege und Brücken

Weg, Straße oder Brücke auswählen und mit gedrückter linker Maustaste ziehen.
Brücken müssen trockene Ufer verbinden. Rechtsklick oder Escape beendet den
Baumodus. Bereits vorhandene Abschnitte desselben Typs werden nicht nochmals
bezahlt. Straßen beschleunigen Einheiten um 50 %, Wege um 20 %, Brücken um 35 %.

## Pferde und Belagerung

Fohlen wachsen im Pferdestall heran. Ein leichter Reiter benötigt zusätzlich
zu den üblichen Kosten ein erwachsenes Pferd. Im Stall können erwachsene Pferde
verkauft werden, wenn ein eigener Markt oder Hafen geöffnet ist.

Die Fabrica baut Ballista, Steinballista, Skorpion, Onager, einfachen und
überdachten Rammbock, Belagerungsturm, Sturmleiter und Pluteus. Geräte kosten
Material, freie Bauern als Besatzung und Arbeitszeit. Waffenforschung Stufe 1
schaltet Steinballista, überdachten Rammbock und Belagerungsturm frei.

Leiter oder Belagerungsturm auswählen und eine gegnerische Befestigung angreifen,
um das Gerät anzulegen. Anschließend eigene Fußtruppen am Boden auswählen und
im Fenster **Mauern erstürmen** den Aufstieg benutzen. Ein Bewegungsbefehl auf den
Boden führt sie über denselben noch vorhandenen Aufstieg zurück. Sturmleitern
reichen bis 8,5 Meter, Belagerungstürme bis 12,5 Meter über dem Standort.
Der Pluteus ist ein beweglicher Geschossschutz und greift nicht selbst an.

## Warnungen

Die Alarmposaune im Palast spielt einen Hornruf. Tatsächliche Angriffe auf eigene
Einheiten oder Gebäude lösen einen anderen Warnton und eine rot blinkende Stelle
auf der Minimap aus. Nach Ende der Schäden klingt die Markierung nach wenigen
Sekunden ab. Der Browser benötigt zunächst einen Klick oder Tastendruck, um Ton
freizugeben.

## Entwicklung und Prüfung

Die `roman-*.test.ts`-Tests verwenden die getrennte Datenbank
`stronghold-roman-test` über `RTS_TEST_DATABASE`. Sie dürfen nicht auf einem
Spielstand des Benutzers laufen. Für den Belagerungsturm wird
`RTS_SIEGE_TOWER=1` beim Test `roman-siege.test.ts` gesetzt. Bestehende Spielstände
bleiben über zusätzliche private Speichertabellen kompatibel.
