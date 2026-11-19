# aeivrix.github.io

Personal page, live at https://aeivrix.github.io.

One static `index.html`, with no framework and no build step for the page itself.

- `blossom/`: a Rust crate that grows the blossom branches from a random seed and outputs canvas draw commands. It compiles to `blossom.wasm`.
- `contrib.json`: contribution data for the heatmap, refreshed daily by `.github/workflows/contrib.yml`.
- Merged PRs and repos are fetched from the GitHub API in the browser.

Press `~` on the page for a shell.

## Rebuilding the blossoms

```sh
cd blossom
cargo test
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/blossom.wasm ../
```

## Running locally

```sh
python3 -m http.server
```

Open http://localhost:8000. The page needs to be served over HTTP because it fetches `blossom.wasm` and `contrib.json`.
