<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.svg">
  <img alt="sentrux" src="assets/logo-dark.svg" width="200">
</picture>

<br><br>

**Dein AI-Agent schreibt den Code.<br>sentrux zeigt dir die Architektur und bewertet die Qualität — live.**

<br>

[![CI](https://github.com/sentrux/sentrux/actions/workflows/ci.yml/badge.svg)](https://github.com/sentrux/sentrux/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/sentrux/sentrux)](https://github.com/sentrux/sentrux/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)


[English](README.md) | [中文](README.zh-CN.md) | **Deutsch** | [日本語](README.ja.md)

[Installation](#installation) · [Schnellstart](#schnellstart) · [MCP-Integration](#mcp-server) · [Regel-Engine](#regel-engine) · [Releases](https://github.com/sentrux/sentrux/releases)

</div>

<br>

<div align="center">

![sentrux Demo](assets/demo.gif)

</div>

<div align="center">
<sub>Ein Prompt. Ein AI-Agent. Fünf Minuten. <b>Health Grade: D.</b></sub>
<br>
<sub>Sieh zu, wie Claude Code ein FastAPI-Projekt von Grund auf erstellt — während sentrux den Architekturverfall in Echtzeit zeigt.</sub>
</div>

<details>
<summary>Vollständigen Bewertungsbericht dieses Demo-Projekts anzeigen</summary>
<br>
<table>
<tr>
<td align="center"><img src="assets/grade-health.png" width="240" alt="Health Grade D"><br><b>Gesundheit: D</b><br><sub>Kohäsion F, toter Code F (25%)<br>Kommentare D (2%)</sub></td>
<td align="center"><img src="assets/grade-architecture.png" width="240" alt="Architecture Grade B"><br><b>Architektur: B</b><br><sub>Schichtung A, Distanz A<br>Blast-Radius B (23 Dateien)</sub></td>
<td align="center"><img src="assets/grade-test-coverage.png" width="240" alt="Test Coverage Grade D"><br><b>Testabdeckung: D</b><br><sub>38% Abdeckung<br>42 ungetestete Dateien</sub></td>
</tr>
</table>
</details>

<br>

## Das Problem, über das niemand spricht

Du startest ein Projekt mit Claude Code oder Cursor. Tag eins ist magisch. Der Agent schreibt sauberen Code, versteht deine Absicht, liefert Features schnell.

Dann verschiebt sich etwas.

Der Agent beginnt, Funktionen zu halluzinieren, die nicht existieren. Er legt neuen Code an der falschen Stelle ab. Er führt Bugs in Dateien ein, die er gestern bearbeitet hat. Du fragst nach einem einfachen Feature und er zerstört drei andere Dinge. Du verbringst mehr Zeit damit, die Ausgabe des Agents zu reparieren, als selbst zu programmieren.

Alle nehmen an, die KI sei schlechter geworden. **Ist sie nicht.** Deine Codebasis schon.

Folgendes ist passiert: Als du eine IDE benutzt hast, sahst du den Dateibaum. Du hast Dateien geöffnet. Du hast ein mentales Modell der Architektur aufgebaut — welches Modul was macht, wie sie verbunden sind, wo Dinge hingehören. Du warst der Regler. Jede Änderung durchlief dein Verständnis des Ganzen.

Dann haben AI-Agents uns ins Terminal verlagert. Der Agent ändert Dutzende von Dateien pro Sitzung. Du siehst einen Strom von `Modified src/foo.rs` — aber du hast das räumliche Bewusstsein verloren. Du siehst nicht, wo diese Datei im Abhängigkeitsgraphen sitzt. Du siehst nicht, dass gerade ein Zyklus entstanden ist. Du siehst nicht, dass drei Module jetzt von einer Datei abhängen, die intern sein sollte. Viele Entwickler lassen AI-Agents ganze Anwendungen bauen, ohne jemals den Dateibrowser zu öffnen.

**Du hast die Kontrolle verloren. Und du weißt es nicht einmal.**

Jede AI-Sitzung degradiert stillschweigend deine Architektur. Gleiche Funktionsnamen, verschiedene Zwecke, verstreut über Dateien. Zusammenhangloser Code im selben Ordner abgelegt. Abhängigkeiten verheddern sich zu Spaghetti. Wenn der Agent dein Projekt durchsucht, findet er zwanzig widersprüchliche Treffer — und wählt den falschen. Jede Sitzung verschlimmert das Chaos. Jedes Chaos erschwert die nächste Sitzung.

Das schmutzige Geheimnis der KI-gestützten Entwicklung: **Je besser die KI Code generiert, desto schneller wird deine Codebasis unregierbar.**

Die traditionelle Antwort — *„plane zuerst deine Architektur, dann lass die KI implementieren"* — klingt richtig, verfehlt aber den Punkt. Tools wie GitHubs [Spec Kit](https://github.com/github/spec-kit) versuchen genau das: detaillierte Spezifikationen und Pläne generieren, bevor Code geschrieben wird. Aber in der Praxis [erfindet es den Wasserfall neu](https://blog.scottlogic.com/2025/11/26/putting-spec-kit-through-its-paces-radical-idea-or-reinvented-waterfall.html) — es produziert Berge von Markdown-Dokumenten, hat aber keinerlei Einblick in den tatsächlich erzeugten Code. Keine Rückkopplungsschleife. Keine Möglichkeit zu erkennen, wann die Implementierung von der Spezifikation abweicht. Keinerlei Strukturanalyse. Die Spec geht rein, der Agent schreibt Code, und niemand prüft, was dabei herauskam.

So arbeitet ohnehin niemand wirklich mit AI-Agents. Du prototypisierst schnell. Du iterierst im Gespräch. Du folgst der Inspiration. Du lässt den kreativen Fluss den Code treiben. Dieser kreative Fluss ist genau das, was AI-Agents mächtig macht. Und genau das, was Codebasen zerstört.

**Du brauchst keinen besseren Plan. Du brauchst einen besseren Sensor.**

## Die Lösung

**sentrux ist die fehlende Rückkopplungsschleife.**

Jedes System, das im großen Maßstab funktioniert, hat eine: einen Sensor, der die Realität beobachtet, eine Spezifikation, die „gut" definiert, und einen Aktor, der Abweichungen korrigiert. Compiler schließen eine Rückkopplungsschleife bei der Syntax. Testsuiten bei Verhalten. Linter beim Stil.

Aber Architektur — passt diese Änderung zum System? Wird diese Abstraktion Probleme verursachen? — hatte keinen Sensor und keinen Aktor. Nur Menschen konnten das beurteilen, und Menschen können mit maschineller Code-Generierung nicht mithalten.

**sentrux schließt die Schleife auf Architekturebene.**

Es beobachtet deine Codebasis in Echtzeit — nicht die Diffs, nicht die Terminal-Ausgabe — die *tatsächliche Struktur*. Jede Datei. Jede Abhängigkeit. Jede architektonische Beziehung. Visualisiert als interaktive Live-Treemap, die sich aktualisiert, während der Agent Code schreibt.

14 Gesundheitsdimensionen. Benotet von A bis F. Berechnet in Millisekunden.

Wenn die Architektur degradiert, siehst du es sofort — nicht zwei Wochen später, wenn alles kaputt ist und niemand sich erinnert, welche Sitzung es verursacht hat.

sentrux gibt dir den Sensor. Deine Regeln geben die Spezifikation. Der Agent ist der Aktor. **Die Schleife schließt sich.**

<br>

<div align="center">
<table>
<tr>
<td align="center" width="33%"><b>Visualisieren</b><br><sub>Live-Treemap mit Abhängigkeitskanten,<br>Dateien leuchten bei Änderungen</sub></td>
<td align="center" width="33%"><b>Messen</b><br><sub>14 Gesundheitsdimensionen A-F:<br>Kopplung, Zyklen, Kohäsion, toter Code…</sub></td>
<td align="center" width="33%"><b>Steuern</b><br><sub>Quality Gate fängt Regression ab.<br>Regel-Engine erzwingt Vorgaben.</sub></td>
</tr>
</table>
</div>

<br>

## Installation

```bash
brew install sentrux/tap/sentrux
```

```bash
# oder: macOS / Linux
curl -fsSL https://raw.githubusercontent.com/sentrux/sentrux/main/install.sh | sh
```

Pures Rust. Einzelne Binary. Keine Laufzeitabhängigkeiten. 23 Sprachen via tree-sitter.

<details>
<summary>Aus Quellcode bauen / Upgrade</summary>

```bash
# Aus Quellcode bauen
git clone https://github.com/sentrux/sentrux.git
cd sentrux && cargo build --release

# Upgrade
brew update && brew upgrade sentrux
# oder curl-Installation erneut ausführen — holt immer die neueste Version
```

</details>

## Schnellstart

```bash
sentrux                    # GUI öffnen — Live-Treemap deines Projekts
sentrux check .            # Regeln prüfen (CI-freundlich, Exit-Code 0 oder 1)
sentrux gate --save .      # Baseline vor Agent-Sitzung speichern
sentrux gate .             # Danach vergleichen — Degradation erkennen
```

## MCP-Server

sentrux läuft als [MCP](https://modelcontextprotocol.io)-Server — dein AI-Agent kann die strukturelle Gesundheit während der Sitzung abfragen.

```json
{
  "sentrux": {
    "command": "sentrux",
    "args": ["--mcp"]
  }
}
```

Funktioniert mit Claude Code, Cursor, Windsurf und jedem MCP-kompatiblen Agent.

<details>
<summary>Agent-Workflow anzeigen</summary>

```
Agent: scan("/Users/me/myproject")
  → { structure_grade: "B", architecture_grade: "B", files: 139 }

Agent: session_start()
  → { status: "Baseline saved", grade: "B" }

  ... Agent schreibt 500 Zeilen Code ...

Agent: session_end()
  → { pass: false, grade_before: "B", grade_after: "C",
      summary: "Architecture degraded during this session" }
```

15 Tools: `scan` · `health` · `architecture` · `coupling` · `cycles` · `hottest` · `evolution` · `dsm` · `test_gaps` · `check_rules` · `session_start` · `session_end` · `rescan` · `blast_radius` · `level`

</details>

## Regel-Engine

Definiere Architekturvorgaben. Erzwinge sie in der CI. Lass den Agent die Grenzen kennen.

<details>
<summary>Beispiel .sentrux/rules.toml</summary>

```toml
[constraints]
max_cycles = 0
max_coupling = "B"
max_cc = 25
no_god_files = true

[[layers]]
name = "core"
paths = ["src/core/*"]
order = 0

[[layers]]
name = "app"
paths = ["src/app/*"]
order = 2

[[boundaries]]
from = "src/app/*"
to = "src/core/internal/*"
reason = "App darf nicht von core-Interna abhängen"
```

```bash
sentrux check .
# ✓ Alle Regeln bestanden — Struktur: B  Architektur: B
```

</details>

## Unterstützte Sprachen

Rust · Python · JavaScript · TypeScript · Go · C · C++ · Java · Ruby · C# · PHP · Bash · HTML · CSS · SCSS · Swift · Lua · Scala · Elixir · Haskell · Zig · R · OCaml

---

## Philosophie

**Die Rolle des Menschen wandelt sich — vom Code-Schreiben zum Code-Regieren.**

Jede Ingenieurspraxis, die vor der KI wichtig war — Dokumentation, Tests, kodifizierte Architektur, schnelle Feedback-Schleifen — ist jetzt exponentiell wichtiger. Überspringe die Tests und die Feedback-Schleife kann sich nicht schließen. Überspringe die Architekturvorgaben und Drift verstärkt sich mit Maschinengeschwindigkeit. Und hier ist die Falle: Du kannst Agents nicht nutzen, um das Chaos aufzuräumen, wenn die Agents nicht wissen, wie „aufgeräumt" aussieht.

sentrux basiert auf drei Überzeugungen:

**1. Human-in-the-Loop ist nicht verhandelbar.** AI-Agents sind mächtig, aber begrenzt. Sie können das große Bild und die kleinen Details nicht gleichzeitig im Blick behalten. Ein Mensch muss jederzeit sehen können, was der Agent mit dem Ganzen macht — nicht nur welche Datei er geändert hat, sondern was diese Datei für die Architektur bedeutet. sentrux macht das möglich.

**2. Verifikation ist wertvoller als Generierung.** Eine korrekte Lösung zu generieren ist schwerer als eine zu verifizieren (die Intuition hinter P vs NP). Du musst die Maschine nicht im Programmieren übertreffen. Du musst sie im Bewerten übertreffen — definieren, wie „korrekt" aussieht, erkennen, wenn die Ausgabe daneben liegt, beurteilen, ob die Richtung stimmt. sentrux verwandelt architektonisches Urteilsvermögen in maschinenlesbare Noten und Vorgaben.

**3. Gute Systeme machen gute Ergebnisse unvermeidlich.** Ein gut entworfenes System schränkt Verhalten so ein, dass das Richtige das Einfache ist. Ein Quality Gate, das Degradation vor der Auslieferung blockiert. Eine Regel-Engine, die deine Architekturentscheidungen kodifiziert. Eine visuelle Karte, die strukturelle Fäulnis unmöglich zu übersehen macht. Die Praktiken haben sich nicht geändert. Die Strafe für ihre Missachtung ist unerträglich geworden.

*Wenn du einmal eine funktionierende Rückkopplungsschleife hast, gehst du nicht zurück zum manuellen Arbeiten. Nicht weil du es nicht kannst. Weil es keinen Sinn mehr ergibt.*

---

<div align="center">

<sub>AI-Agents schreiben Code mit Maschinengeschwindigkeit. Ohne strukturelle Governance verfallen Codebasen mit Maschinengeschwindigkeit.<br><b>sentrux ist der Regler.</b></sub>

</div>

## Lizenz

[MIT](LICENSE)
