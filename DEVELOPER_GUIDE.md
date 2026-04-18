# FEMReader Developer Guide

Esta guía describe la arquitectura interna del proyecto y cómo extenderla sin romper el flujo actual.

## 1. Objetivo técnico

FEMReader separa claramente:

1. descubrimiento de archivos FEM,
2. resolución de dependencias entre archivos,
3. ensamblado de modelos lógicos,
4. serialización para consumo externo.

La meta es poder ampliar reglas de parsing/solver y salidas sin duplicar lógica central.

## 2. Estructura del código (`src/`)

- `lib.rs`: expone módulos públicos.
- `main.rs`: entrada CLI.
- `bin/femreader_gui.rs`: entrada GUI.
- `scanner.rs`: recorre el filesystem y genera nodos de directorio/archivo.
- `dependency.rs`: detecta referencias (`INCLUDE`, etc.) y construye el grafo solo con archivos de modelo.
- `assembler.rs`: agrupa archivos conectados en modelos (`Model`).
- `solver.rs`: trait de adaptadores + implementaciones iniciales (Nastran/OptiStruct/Abaqus).
- `optistruct.rs`: parser de tarjetas OptiStruct (campos, continuaciones, comentarios y tipado de tarjetas clave).
- `serializer.rs`: serialización JSON compacta o pretty.
- `cache.rs`: cache incremental por path, tamaño y fecha de modificación.
- `pipeline.rs`: orquestación scanner -> dependency -> assembler -> serializer.
- `model.rs`: entidades de dominio y transporte (`ModelContent`, `Model`, `FileNode`, etc.).

## 3. Flujo end-to-end

1. **Scanner** (`DefaultFileSystemScanner`) recorre el árbol de directorios y crea:
   - `directories: Vec<DirectoryNode>`
   - `files: Vec<FileNode>` (con extensión, tamaño, timestamp, estado de cambio y categoría `Model`/`NonModel`)
2. **DependencyResolver** (`DefaultDependencyResolver`) hace:
   - detección de solver por directorio;
   - lectura de contenido solo para archivos `Model` soportados por adaptadores;
   - extracción de referencias y resolución a paths canónicos;
   - lectura estructurada de OptiStruct (`optistruct_items`) con metadatos de archivo/línea.
   - construcción de `DependencyGraph`.
3. **Assembler** (`DefaultModelAssembler`) detecta raíces del grafo y arma modelos solo con archivos `Model`.
4. **Serializer** (`JsonSerializer`) genera JSON de salida.

`ReaderPipeline` encapsula este flujo para reutilizarlo desde CLI y GUI.

### Puntos de entrada de lectura de archivos

- Entrada end-to-end: `ReaderPipeline::run` en `src/pipeline.rs`.
- Entrada de lectura de filesystem: `DefaultFileSystemScanner::scan` en `src/scanner.rs`.
- Entrada de lectura de contenido de archivos para parsing/referencias: `DefaultDependencyResolver::resolve` en `src/dependency.rs` (bloque `fs::read_to_string(&file.path)`).

## 4. Identificación automática de solver

La identificación actual se aplica **por directorio** en `dependency.rs`:

- si el directorio contiene `.fem` (y no `.bdf`/`.inp`) -> `OptiStruct`
- si contiene `.bdf` (y no `.fem`/`.inp`) -> `NASTRAN`
- si contiene `.inp` (y no `.fem`/`.bdf`) -> `ABAQUS`
- si contiene combinación de dos o más -> `Unknown`

Notas:

- Solo se consideran `.fem`, `.bdf`, `.inp` para inferencia.
- El ensamblador marca el solver del modelo como `Unknown` si dentro del modelo conviven hints incompatibles.

## 5. Adaptadores de solver

`SolverAdapter` define:

- `supports_file(path)`: si el parser aplica al archivo.
- `extract_references(content)`: extracción de referencias de include.

Actualmente se usan para resolver dependencias, no para decidir solver de forma final.

### OptiStruct (lógica de lectura de tarjetas)

`src/optistruct.rs` implementa un parser base:

- tarjetas en formato corto (hasta 10 campos, 8 caracteres por campo),
- formato largo (16 caracteres por campo),
- continuidad de tarjeta con línea que inicia por `*`,
- identificación de comentarios con `$` (línea completa o inline).

Además del `INCLUDE`, actualmente se tipan:

- `GRID`
- `CQUAD4`
- `CTRIA3`
- `FORCE`
- `MOMENT`

Cada item parseado (`Comment` o `Card`) incluye:

- archivo (`file_id`, `file_path`),
- posición (`start_line`, `end_line`),
- orden estable dentro del archivo (`ordinal`),
- campos crudos y versión tipada de la tarjeta.

Guía específica para extender tarjetas:

- `OPTISTRUCT_CARDS_GUIDE.md`

Para añadir un nuevo solver:

1. crear adapter en `solver.rs`,
2. añadirlo al vector de adaptadores en `main.rs` y `bin/femreader_gui.rs`,
3. (opcional) ampliar reglas de inferencia por extensión en `dependency.rs`.

## 6. GUI (`bin/femreader_gui.rs`)

La GUI está implementada con `eframe/egui` y reutiliza el mismo pipeline:

- carga un directorio de entrada;
- muestra métricas de contenido (`models`, `files`, `dependencies`) y diferenciación `Model`/`NonModel`;
- lista modelos con solver detectado;
- permite inspeccionar archivos del modelo y dependencias relacionadas;
- muestra referencias no resueltas;
- muestra la estructura de items OptiStruct por archivo (comentarios + tarjetas con líneas y campos);
- permite borrar la cache tras la carga.

Diseño actual: visor de inspección y depuración. No modifica archivos del modelo.

Compilación/ejecución:

```bash
cargo run --features gui --bin femreader_gui -- /ruta/absoluta/a/modelos
```

## 7. Tests

Los tests de integración están en `tests/integration_reader.rs` y cubren:

- ensamblado de múltiples modelos con dependencia compartida;
- comportamiento del cache incremental;
- inferencia de solver por extensión/directorio;
- caso mixto de extensiones incompatibles (`Unknown`).
- separación entre archivos de modelo y no modelo;
- filtrado del grafo para relacionar solo archivos de modelo;
- borrado opcional de `.femreader-cache`.
- parsing de tarjetas OptiStruct con metadatos de posición y tipado de tarjetas clave.

`src/optistruct.rs` incluye tests unitarios del parser de tarjetas y comentarios.

Ejecutar:

```bash
cargo test
```

## 8. Extensiones recomendadas

- extraer factoría compartida para construir el `ReaderPipeline` (evitar duplicación entre CLI/GUI/tests),
- soportar parsing FEM más rico (`Node`, `Element`, `Material`, etc.),
- incorporar visualización de grafo de dependencias en la GUI.
