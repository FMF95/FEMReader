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
- `dependency.rs`: detecta referencias (`INCLUDE`, etc.) y construye el grafo.
- `assembler.rs`: agrupa archivos conectados en modelos (`Model`).
- `solver.rs`: trait de adaptadores + implementaciones iniciales (Nastran/OptiStruct/Abaqus).
- `serializer.rs`: serialización JSON compacta o pretty.
- `cache.rs`: cache incremental por path, tamaño y fecha de modificación.
- `pipeline.rs`: orquestación scanner -> dependency -> assembler -> serializer.
- `model.rs`: entidades de dominio y transporte (`ModelContent`, `Model`, `FileNode`, etc.).

## 3. Flujo end-to-end

1. **Scanner** (`DefaultFileSystemScanner`) recorre el árbol de directorios y crea:
   - `directories: Vec<DirectoryNode>`
   - `files: Vec<FileNode>` (con extensión, tamaño, timestamp, estado de cambio)
2. **DependencyResolver** (`DefaultDependencyResolver`) hace:
   - detección de solver por directorio;
   - lectura de contenido de archivos soportados por adaptadores;
   - extracción de referencias y resolución a paths canónicos;
   - construcción de `DependencyGraph`.
3. **Assembler** (`DefaultModelAssembler`) detecta raíces del grafo y arma modelos.
4. **Serializer** (`JsonSerializer`) genera JSON de salida.

`ReaderPipeline` encapsula este flujo para reutilizarlo desde CLI y GUI.

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

Para añadir un nuevo solver:

1. crear adapter en `solver.rs`,
2. añadirlo al vector de adaptadores en `main.rs` y `bin/femreader_gui.rs`,
3. (opcional) ampliar reglas de inferencia por extensión en `dependency.rs`.

## 6. GUI (`bin/femreader_gui.rs`)

La GUI está implementada con `eframe/egui` y reutiliza el mismo pipeline:

- carga un directorio de entrada;
- muestra métricas de contenido (`models`, `files`, `dependencies`);
- lista modelos con solver detectado;
- permite inspeccionar archivos del modelo y dependencias relacionadas;
- muestra referencias no resueltas.

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

Ejecutar:

```bash
cargo test
```

## 8. Extensiones recomendadas

- extraer factoría compartida para construir el `ReaderPipeline` (evitar duplicación entre CLI/GUI/tests),
- soportar parsing FEM más rico (`Node`, `Element`, `Material`, etc.),
- incorporar visualización de grafo de dependencias en la GUI.
