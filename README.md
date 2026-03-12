<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.svg">
  <img alt="sentrux" src="assets/logo-dark.svg" width="200">
</picture>

<br><br>

**See your codebase. Govern your AI agents.**

Live architecture visualization + structural quality gate for AI-agent-written code.

[![CI](https://github.com/sentrux/sentrux/actions/workflows/ci.yml/badge.svg)](https://github.com/sentrux/sentrux/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/sentrux/sentrux)](https://github.com/sentrux/sentrux/releases)

</div>

![sentrux screenshot](assets/screenshot.png)

## Why

In the AI agent era, code is written faster than humans can review. AI agents modify dozens of files per session — you see a stream of "Modified src/foo.rs" but lose the big picture: how files relate, where coupling grows, when architecture degrades.

**sentrux closes that gap.** It gives you a live visual map of your codebase with real-time health grades, so you can see what the agent is doing to your architecture — not just which files it touched.

- **For developers using AI agents** (Claude Code, Cursor, Copilot): watch your architecture in real-time while the agent codes
- **For tech leads**: enforce structural constraints before code ships
- **For anyone inheriting a codebase**: understand the structure in seconds, not hours

## What it does

**Visualize**
- Treemap + Blueprint DAG layouts — files sized by lines, colored by language/heat/complexity
- Dependency edges — import, call, and inheritance as animated polylines
- Real-time file watcher — files glow when modified, incremental rescan

**Measure**
- 14 health dimensions — coupling, cycles, cohesion, entropy, complexity, duplication, dead code (A-F grades)
- 4 architecture metrics — levelization, blast radius, attack surface, distance from main sequence
- Evolution analysis — git churn, bus factor, temporal hotspots
- DSM (Design Structure Matrix) — NxN dependency matrix with cluster detection
- Test gap analysis — find high-risk untested files

**Govern**
- Rules engine — define architectural constraints in `.sentrux/rules.toml`
- Baseline gate — `sentrux gate` catches structural regression before it ships
- MCP server — 15 tools for AI agent integration

## Install

### Homebrew (macOS)

```bash
brew install sentrux/tap/sentrux
```

### Quick install (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/sentrux/sentrux/main/install.sh | sh
```

### From source

```bash
git clone https://github.com/sentrux/sentrux.git
cd sentrux
cargo build --release
# Binary at target/release/sentrux
```

Or download binaries from [Releases](https://github.com/sentrux/sentrux/releases).

## Quick start

```bash
# Open the GUI — visual treemap of your project
sentrux

# Check architectural rules (CI-friendly, exits 0 or 1)
sentrux check /path/to/project

# Structural regression gate
sentrux gate --save .   # save baseline before agent session
sentrux gate .          # compare after — catches degradation
```

## MCP server (AI agent integration)

sentrux runs as a [Model Context Protocol](https://modelcontextprotocol.io) server, giving AI agents real-time access to your codebase's structural health.

```bash
sentrux --mcp
```

Add to your `.mcp.json` (Claude Code, Cursor, etc.):

```json
{
  "sentrux": {
    "command": "sentrux",
    "args": ["--mcp"]
  }
}
```

**Example: agent checks health after writing code**

```
Agent: scan("/Users/me/myproject")
  → { structure_grade: "B", architecture_grade: "B", files: 139 }

Agent: health()
  → { grade: "B", dimensions: { coupling: "A", cycles: "A", cohesion: "C", ... } }

Agent: session_start()
  → { status: "Baseline saved", grade: "B" }

  ... agent writes 500 lines of code ...

Agent: session_end()
  → { pass: false, grade_before: "B", grade_after: "C",
      summary: "Architecture degraded during this session" }
```

15 tools available: `scan`, `health`, `architecture`, `coupling`, `cycles`, `hottest`, `evolution`, `dsm`, `test_gaps`, `check_rules`, `session_start`, `session_end`, `rescan`, `blast_radius`, `level`.

## Rules engine

Define architectural constraints in `.sentrux/rules.toml`:

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
reason = "App must not depend on core internals"
```

```bash
sentrux check .
# sentrux check — 4 rules checked
# Structure grade: B  Architecture grade: B
# ✓ All rules pass
```

## Supported languages

Rust, Python, JavaScript, TypeScript, Go, C, C++, Java, Ruby, C#, PHP, Bash, HTML, CSS, SCSS, Swift, Lua, Scala, Elixir, Haskell, Zig, R, OCaml — 23 languages via tree-sitter.

## Architecture

```
sentrux/
├── sentrux-core/    # library crate — analysis engine, metrics, MCP server
│   ├── analysis/    # scanning, parsing, import resolution, graph construction
│   ├── metrics/     # health grading, architecture analysis, DSM, evolution
│   ├── app/         # GUI panels, MCP server, state management
│   ├── layout/      # treemap, blueprint DAG, edge routing, spatial index
│   └── renderer/    # egui rendering (rects, edges, badges, minimap, heat)
└── sentrux-bin/     # binary crate — GUI, CLI, MCP entry points
```

## License

[MIT](LICENSE)
