# FEMReader

Lector de modelos FEM en Rust, orientado a escaneo rápido, resolución de dependencias y salida consistente para integración con otras herramientas.

## Funcionalidades principales

- **Core library** para escaneo de archivos, resolución de referencias y ensamblado de modelos.
- **CLI** para procesar un directorio y exportar JSON versionado.
- **Cache incremental** (`.femreader-cache`) para evitar reprocesar archivos no modificados.
- **GUI de escritorio** para visualizar modelos, archivos y dependencias.
- **Identificación automática de solver por directorio**:
  - `.fem` -> `OptiStruct`
  - `.bdf` -> `NASTRAN`
  - `.inp` -> `ABAQUS`
  - Si en el mismo directorio aparecen dos o más de esas extensiones, el solver se marca como `Unknown`.

## Requisitos

- Rust + Cargo (toolchain estable).

## Uso rápido (CLI)

Imprimir JSON en consola:

```bash
cargo run -- /ruta/absoluta/a/modelos --pretty
```

Guardar JSON en archivo:

```bash
cargo run -- /ruta/absoluta/a/modelos --output /ruta/absoluta/salida.json --pretty
```

Ayuda:

```bash
cargo run -- --help
```

## Uso de la GUI

Lanzar la interfaz gráfica:

```bash
cargo run --features gui --bin femreader_gui -- /ruta/absoluta/a/modelos
```

En la GUI puedes:

- Cargar un directorio de modelos.
- Ver resumen de directorios, archivos, dependencias y referencias sin resolver.
- Seleccionar cada modelo y revisar sus archivos, solver detectado y relaciones de dependencia.

## Contrato de salida JSON

La salida serializada incluye:

- `schema_version`
- `root_directory`
- `directories`
- `files`
- `dependencies`
- `unresolved_references`
- `models`

## Desarrollo y arquitectura

Consulta la guía técnica para desarrolladores:

- `DEVELOPER_GUIDE.md`

## Tests

```bash
cargo test
```
