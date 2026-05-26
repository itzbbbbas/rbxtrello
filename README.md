# rbxtrello

Declarative Trello-board sync from a single TOML file. Sibling to [rbxmonet](https://github.com/itzbbbbas/rbxmonet) — same install flow, same UX.

Treat your Trello board as out-of-game documentation (a player wiki). Edit `rbxtrello.toml`, run `rbxtrello sync`, board state matches.

---

## Install

Via [Rokit](https://github.com/rojo-rbx/rokit) (recommended):

```toml
# rokit.toml
[tools]
rbxtrello = "itzbbbbas/rbxtrello@0.2.0"
```

```sh
rokit install
```

Or build from source:

```sh
git clone https://github.com/itzbbbbas/rbxtrello
cd rbxtrello
cargo install --path .
```

---

## Auth

Two env vars (or `.env` next to the binary):

```sh
TRELLO_KEY=<your-key>      # https://trello.com/app-key
TRELLO_TOKEN=<your-token>  # generate on same page
```

---

## Commands

```
rbxtrello init                      Write starter rbxtrello.toml in cwd
rbxtrello pull                      Fetch remote board → overwrite rbxtrello.toml
rbxtrello sync [-a|--auto-confirm]  Diff local→remote, TUI confirm, push changes
                                    Bypass TUI: -a or RBXTRELLO_AUTO_CONFIRM=1

Global flags:
  -y, --yes              Auto-confirm all prompts
  --dry-run              Print diff, make no changes
  --board-id <id>        Override [metadata].board_id
```

---

## Schema

```toml
[metadata]
board_name = "My Game Wiki"
board_id   = "abc123"               # written by pull/sync; omit on first sync to create board

# Reusable labels — referenced by slug on cards
[labels.common]    ; color = "sky"   ; name = "Common"
[labels.rare]      ; color = "blue"  ; name = "Rare"
[labels.tradable]  ; color = "green" ; name = "Tradable"
# Valid colors: yellow, purple, blue, red, green, orange, black, sky, pink, lime

# Lists
[lists.brainrots]
name     = "Brainrots"
position = 2
managed  = true                     # default; false = ignore orphans on this list

[lists.mechanics]
name     = "Mechanics"
position = 1
managed  = false                    # human-curated; never archive orphans

# Cards — keyed by stable slug; Trello card ID written back inline after sync
[lists.brainrots.cards.noobini_pizzanini]
name     = "Noobini Pizzanini"
desc     = "Common starter brainrot..."   # verbatim markdown
labels   = ["common", "tradable"]
complete = true                           # marks Trello's dueComplete badge (default: false, omitted when false)
# id     = "5fab..."                  # filled by sync/pull
```

---

## Typical flows

### First-time bootstrap of an existing board

```sh
rbxtrello init                       # writes template
# edit rbxtrello.toml: set [metadata].board_id = "<your-existing-board>"
rbxtrello pull                       # overwrites local toml with remote state
git diff rbxtrello.toml              # review
```

### Day-to-day

```sh
# edit rbxtrello.toml — add/rename/remove cards
rbxtrello sync                       # TUI shows diff, press Y to apply
```

### Unattended / CI

```sh
RBXTRELLO_AUTO_CONFIRM=1 rbxtrello sync
# or
rbxtrello sync --auto-confirm
```

---

## What's new in v0.2.0

- **`complete` field on cards** — set `complete = true` on a card and `sync` will toggle Trello's `dueComplete` flag (green checkmark badge). Omit or set `false` to clear it. `pull` reads the flag back from remote.

---

## Limitations (v0.2.0)

- Card cover images, checklists, and custom fields are declared in schema but not yet pushed by `sync` (planned for a future release).
- Single board per toml.
- No deletion — orphans are archived (Trello-side), recoverable via the board's archive.

---

## Logging

`env_logger` reads `RUST_LOG`:

```sh
RUST_LOG=rbxtrello=debug rbxtrello sync
```

Default: `rbxtrello=info` (release), `rbxtrello=debug` (debug).
