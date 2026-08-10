# Copilot Instructions

## Project shape

- The useful application logic in this repo is the Python generator script, `generate-address-list.py`.
- The Rust crate is currently a scaffold: `src/lib.rs` contains a placeholder test, and `src/main.rs` is empty.
- The GitHub Actions workflow in `.github/workflows/generate-lists.yml` is the automation path that downloads OFAC data, generates address lists, and updates the `lists` branch.

## Build, test, and lint

- Rust test suite: `cargo test`
- Single Rust test: `cargo test it_works --lib`
- Rust format check: `cargo fmt --check`
- Rust compile check: `cargo check` (currently fails until `src/main.rs` has a `main` function)
- Python help/CLI check: `python3 generate-address-list.py --help`
- Fetch the OFAC XML: `python3 generate-address-list.py fetch -o sdn_advanced.xml`
- Generate lists from a local XML file: `python3 generate-address-list.py XBT ETH -sdn sdn_advanced.xml -f TXT JSON -path ./out`

## Architecture

- `generate-address-list.py` reads the OFAC SDN XML, finds the `FeatureType` for each supported asset, extracts matching `VersionDetail` values, deduplicates and sorts them, then writes `sanctioned_addresses_<ASSET>.txt` and/or `.json`.
- The script now has two modes: `fetch` downloads the published OFAC ZIP and extracts the XML, and the default mode generates address lists from an existing XML file.
- The workflow uses a local virtual environment (`.venv`) and runs the Python script directly; it fetches `SDN_ADVANCED.XML`, writes outputs into `data/`, then moves them into the `lists` branch commit.

## Conventions

- Keep asset handling constrained to the existing `POSSIBLE_ASSETS` list unless the OFAC XML format changes.
- Prefer `pathlib.Path` for file and directory arguments.
- Preserve the current output naming scheme: `sanctioned_addresses_<ASSET>.txt` and `sanctioned_addresses_<ASSET>.json`.
- Use lowercase `sdn_advanced.xml` for local files; the workflow uses `SDN_ADVANCED.XML` for the fetched archive contents.
- Treat generated files (`sanctioned_addresses_*`, `sdn_advanced.xml`, `SDN_ADVANCED.XML`, `.venv`, `target/`) as build artifacts, not source.
