# saumfar

A TUI browser and downloader for Geonorge's INSPIRE Atom Download Service feed.

*"Saumfar"* gjennom Geonorge sine datasett — fills the gap left by Geonorge's Windows-only desktop client.

## Install

```bash
cargo install --path .
```

## Usage

### Interactive (TUI)

```bash
saumfar
```

lazygit-style interface for browsing all datasets in the INSPIRE Atom service feed.

| Key | Action |
|-----|--------|
| `/` or type | Fuzzy search |
| `s` | Cycle sort column |
| `f` | Toggle filter panel |
| `Enter` | Drill into dataset |
| `Esc` | Back |
| `d` | Download selected |
| `q` | Quit |

### Non-interactive

```bash
# List datasets
saumfar list --search "FKB" --owner "Kartverket"

# Download a dataset
saumfar download <dataset-id> --format SOSI --output-dir ./data
```

## Feed structure

The [INSPIRE Atom Download Service](https://nedlasting.geonorge.no/geonorge/Tjenestefeed.xml) exposes a two-level Atom feed:

1. **Service feed** — one entry per dataset (title, owner, CRS, last updated)
2. **Dataset feed** — per-dataset download entries (format, area, direct URL)

## Stack

Rust, ratatui, reqwest, quick-xml, tokio, clap.

## License

MIT
