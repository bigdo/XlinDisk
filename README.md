# XlinDisk

XlinDisk is a cross-platform desktop disk-management tool built around fast,
inspectable filesystem checks and an MCP-oriented interface.

The first milestone is a safe duplicate-file workflow:

1. scan one or more user-provided paths;
2. group candidates using cheap metadata filters followed by content hashes;
3. expose evidence for every match and a deterministic keep/delete proposal;
4. support dry-run, review, quarantine, and undo-friendly deletion flows.

## Why this project

Existing projects provide strong foundations, but leave room for a tool whose
filesystem actions are explicit MCP capabilities with strict path scope,
human-readable evidence, and safe-by-default mutations:

- [Czkawka / Krokiet](https://github.com/qarmin/czkawka) — fast, Rust-based,
  cross-platform duplicate and unnecessary-file finder with GUI and CLI modes.
- [dupeGuru](https://github.com/arsenetar/dupeguru) — desktop-oriented,
  cross-platform duplicate finder with content and fuzzy matching workflows.
- [jdupes](https://github.com/jbruchon/jdupes) — efficient CLI duplicate
  finder with deletion and hard-link actions.
- [rmlint](https://github.com/sahib/rmlint) — filesystem linter with duplicate
  detection, caching, replayable output, and multiple action formats.
- [rdfind](https://github.com/pauldreik/rdfind) — small CLI utility for finding
  duplicate files across directories.
- [MCP filesystem server](https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem)
  — reference for MCP filesystem resources and scoped directory access.

## Initial design principles

- Read-only scan by default; mutations require an explicit confirmation step.
- Never delete based on a hash alone: verify size, hash, and (where needed)
  byte-for-byte equality.
- Treat symlinks, hard links, permissions, mount boundaries, and concurrent
  changes as first-class cases.
- Prefer quarantine/restore over irreversible deletion.
- Return structured, auditable results suitable for both a desktop UI and MCP
  clients.

## Status

**Architecture Draft r1** — direction accepted, in Semantic Contract Freeze.

The core model is not `Entry -> hash -> result`; it is
`Entry -> Observation -> operation -> re-observation -> validation -> result`,
so filesystem scans, duplicate hashing, cache and future deletion all share one
notion of "the object may have changed under us".

- `docs/contracts/` — frozen semantics: object identity, observation
  consistency, locator and source contract, run status/errors, determinism,
  duplicate resource model, result schema, cross-platform fixtures.
  **PR3 must not start until these are merged.**
- `docs/spec/filesystem-semantics.md` — symlink / reparse / mount boundary /
  special files / sparse / ordering rules.
- `docs/adr/` — ADR-001…017.
- `docs/ARCHITECTURE.md` — layout, dependency direction, PR order.
- `crates/` — `xlindisk-core` (frozen types), `xlindisk-source-fs` (skeleton
  until PR3), `xlindisk-cli`, `xlindisk-bench`.

v0.0.1 is a read-only engine: no delete / move / trash in the core.
