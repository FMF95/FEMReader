# FEMReader

Finite element model reader focused on speed, adaptability, and consistency.

## Current implementation

This repository now includes a Rust implementation with:

- **Core library** for filesystem scanning, dependency resolution, and model assembly.
- **CLI executable** that accepts a directory as input and emits a versioned JSON output.
- **Incremental cache** (`.femreader-cache`) to avoid reprocessing unchanged files.
- **Solver adapter architecture** with initial adapters for:
  - NASTRAN
  - OptiStruct
  - ABAQUS

## Architecture

The code is organized in modules under `/home/runner/work/FEMReader/FEMReader/src`:

- `scanner`: filesystem I/O, directory/file mapping, metadata normalization.
- `dependency`: include/reference extraction and dependency graph construction.
- `assembler`: model candidate creation from dependency relationships.
- `solver`: shared adapter trait + solver-specific adapters.
- `serializer`: versioned JSON serialization for inter-language integration.
- `cache`: incremental change tracking by path + metadata.
- `pipeline`: orchestration of scanner -> dependencies -> model assembly.

Main entities include:

- Filesystem entities: `DirectoryNode`, `FileNode`, `DependencyEdge`
- FEM domain entities: `Model`, `ModelContent`
- Transport/output entity: versioned `ModelContent` JSON payload

## CLI usage

```bash
cargo run -- /absolute/path/to/models --pretty
```

Optional output file:

```bash
cargo run -- /absolute/path/to/models --output /absolute/path/to/output.json --pretty
```

## Output contract

The CLI emits a versioned JSON with:

- `schema_version`
- `root_directory`
- `directories`
- `files`
- `dependencies`
- `unresolved_references`
- `models`

This keeps integration simple for other languages through file-based or process-based interoperability.

## Testing

Run:

```bash
cargo test
```

Integration tests cover:

- model detection with shared dependencies
- incremental cache behavior

## Next steps

- Extend solver-specific parsing rules and robustness for real-world decks.
- Add richer FEM content entities (`Element`, `Node`, `Material`, etc.).
- Add GUI/debug viewer consuming the core output (without parse logic duplication).
