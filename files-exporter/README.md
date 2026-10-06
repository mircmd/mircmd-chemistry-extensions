# Files Exporter

Rust WASI Preview 2 component implementing `mircmd:file-exporter@0.1.0`.
The WIT contract is in `wit/file-exporter.wit`.

## Supported formats

| Format ID | File suffix | Supported object types |
| --- | --- | --- |
| `xyz` | `.xyz` | `mircmd:chemistry:atomic_coordinates` |

`formats()` returns the available formats. Each format has a unique ID, a
display name, file suffixes, supported object type IDs and an optional settings
schema. Settings schemas are JSON Schema objects serialized as JSON strings.
Add more format descriptors and dispatch their IDs in `dump` to support multiple
formats in one extension. Compatibility belongs to each format, not to the
extension as a whole.

`dump(data, format_id, settings, file_path)` receives the object's bytes, the
selected format ID, settings serialized as a JSON object and the full output
file path. It writes the file and returns a string error on failure. The host
provides WASI access to the selected output directory; format discovery has no
filesystem access.

## XYZ data and settings

The input is Postcard-serialized atomic coordinates, matching the importer:

```rust
struct AtomicCoordinates {
    atomic_num: Vec<i32>,
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<f64>,
}
```

Coordinates are in angstroms. All four arrays must have equal lengths. Element
numbers 1–118 and the importer's dummy atoms `-1` (`X`) and `-2` (`Q`) are
supported. Non-finite coordinates and trailing bytes are rejected.

`src/xyz.schema.json` describes the settings:

- `title`: the single-line comment on the second line; default `""`.
- `precision`: decimal places, from 0 to 16; default `14`.

Both settings are optional. Missing values use Rust defaults. Unknown settings,
invalid values and malformed data are rejected before opening the output file.

## Build, test and install

```sh
make init
make test
make build
make install
```

No `cargo-component` installation is needed: the `wasm32-wasip2` target produces
a WASI component directly. `make install` copies `manifest.yaml` and the compiled
component as `extension.wasm` to
`~/.config/mircmd/extensions/mircmd/chemistry-files-exporter`. Override
`INSTALL_DIR` to use another location. Restart Mir Commander after installation.

The application uses its own copy of the WIT contract in
`mir_commander_file_exporter_host/wit`; update both copies when changing the
protocol. The application's contract test checks that they match.
