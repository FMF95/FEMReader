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
- `highlighting.rs`: parser ligero de XML UDL (Notepad++) y clasificación de tokens para resaltado en GUI.

## 3. Flujo end-to-end

1. **Scanner** (`DefaultFileSystemScanner`) recorre el árbol de directorios y crea:
   - `directories: Vec<DirectoryNode>`
   - `files: Vec<FileNode>` (con extensión, tamaño, timestamp, estado de cambio y categoría `Model`/`NonModel`)
   - ignora archivos internos de FEMReader como `.femreader-cache`.
   - genera IDs únicos determinísticos para `DirectoryNode.id` y `FileNode.id` (hash estable sin prefijos visibles).
2. **DependencyResolver** (`DefaultDependencyResolver`) hace:
   - detección de solver por directorio;
   - lectura de contenido solo para archivos `Model` soportados por adaptadores;
   - extracción de referencias y resolución a paths canónicos;
   - lectura estructurada de OptiStruct (`optistruct_items`) con metadatos de archivo/línea.
   - resumen por archivo OptiStruct (`optistruct_file_summaries`) con conteos por tipo de ítem/tarjeta.
   - generación de IDs únicos para `DependencyEdge.id` (hash estable sin prefijos visibles).
   - construcción de `DependencyGraph`.
3. **Assembler** (`DefaultModelAssembler`) detecta raíces del grafo y arma modelos solo con archivos `Model`.
   - genera IDs únicos para `Model.id` (hash estable sin prefijos visibles).
   - agrega resumen OptiStruct por modelo (`optistruct_model_summaries`) a partir de los resúmenes por archivo.
4. **Serializer** (`JsonSerializer`) genera JSON de salida.

`ReaderPipeline` encapsula este flujo para reutilizarlo desde CLI y GUI.

### Versionado de esquema JSON

- La versión del contrato JSON se define en un único punto: `model::SCHEMA_VERSION`.
- Si cambia el contrato (keys/estructura/semántica), se debe actualizar `SCHEMA_VERSION`.
- `serializer.rs` incluye un test de guardia de contrato para forzar revisión explícita cuando se modifican aspectos críticos del esquema.

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

- parsing por campos **solo** para tarjetas soportadas explícitamente,
- tarjetas soportadas en formato corto (hasta 10 campos, 8 caracteres por campo),
- soporte de formato largo (16 caracteres por campo),
- continuidad de tarjeta con línea que inicia por `*` (cuando la tarjeta actual es soportada),
- identificación de comentarios con `$` (línea completa o inline).
- detección robusta de `BEGIN BULK` (incluyendo formato libre con espacios variables).
- cualquier línea no comentario sin parser específico se conserva como ítem `Text` (sin forzar parseo por campos).
- en tarjetas, `fields` conserva `F1` como el primer campo (tipo de tarjeta) y desplaza los campos siguientes (`F2..`).

Además del `INCLUDE`, actualmente se tipan:

- `GRID`
- `CQUAD4`
- `CTRIA3`
- `CBUSH`
- `PSHELL`
- `PBUSH`
- `FORCE`
- `MOMENT`
- `BEGIN BULK`
- `ENDDATA`

Comentarios clasificados:

- `Comment`
- `HyperMesh Comment` para prefijos conocidos (`$HMNAME`, `$HWCOLOR`, `$HMMOVE`, `$HMDPRP`).

Cada item parseado (`Comment`, `Text` o `Card`) incluye:

- id único del ítem (`id`),
- archivo (`file_id`, `file_path`),
- nombre de archivo (`file_name`),
- posición (`start_line`, `end_line`),
- orden estable dentro del archivo (`ordinal`),
- campos crudos y versión tipada de la tarjeta.
- en `Card`, `text` preserva la(s) línea(s) original(es) leída(s) para visualización fiel.
- `BEGIN BULK` y `ENDDATA` se modelan como tarjetas sin campos (`fields: []`).

Resumen por archivo (`optistruct_file_summaries`):

- `total_non_empty_lines`
- `covered_non_empty_lines`
- `line_coverage_ok`
- `total_items`
- `comment_items`
- `hypermesh_comment_items`
- `text_items`
- `card_items`
- `card_counts` (conteo por nombre de tarjeta)

Resumen por modelo (`optistruct_model_summaries`):

- mismos indicadores agregados a nivel modelo (`total_*`, `line_coverage_ok`, `card_counts`).

Campos HyperMesh parseados:

- `HMNAME`: `entity_type`, `id`, `name`, `id_pool`
- `HWCOLOR`: `entity_type`, `id`, `color_id`
- `HMMOVE`: `collector_id`
- `HMDPRP`: `elements`, `ids`

Notas de parsing HyperMesh:

- `HMNAME` soporta casos como `1"nombre"` (sin espacio entre `id` y comillas).
- `HMMOVE` soporta continuidad en comentarios siguientes indentados, por ejemplo:
  - `$HMMOVE        1`
  - `$              1THRU         100`
- `HMMOVE` y `HMDPRP` admiten continuidad multi-línea en formato de campos de 8 caracteres (`$` + indentación), acumulando líneas sucesivas hasta que el patrón de continuidad deja de cumplirse.

Guía específica para extender tarjetas:

- `OPTISTRUCT_CARDS_GUIDE.md`
- `DOTNET_INTEGRATION.md` (integración operativa con apps .NET)

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
- muestra la estructura de items OptiStruct por archivo con cabeceras `L# [Tipo] <línea original>`;
- aplica resaltado de sintaxis a las líneas mostradas en `Comment`, `Text` y `Card` usando XML de `highlighting/`;
- alinea visualmente `L# [Tipo]` y texto literal en una disposición tipo dos columnas (cabecera alineada + contenido literal);
- permite seleccionar el fondo del bloque literal resaltado desde la barra superior (`Predeterminado`, `#FFFFFF`, `#FFF8DC`, `#A8A8A8`);
- permite activar/desactivar highlighting desde la barra superior (sin recargar contenido);
- en `NASTRAN/OptiStruct` el bloque literal usa ancho mínimo de 80 caracteres para mantener borde derecho alineado y fondo homogéneo entre líneas;
- cuando el highlighting está activo, el texto sin token específico se dibuja en negro para legibilidad;
- normaliza paths mostrados en GUI eliminando prefijo verbatim de Windows (`\\?\`) para evitar ruido visual;
- en la barra superior, `Cargar` aparece justo tras la ruta y la opción de borrar cache está activada por defecto;
- en tarjetas, muestra campos con nombre semántico (`SID`, `PID`, `X1`, etc.);
- muestra resumen OptiStruct por archivo (y totales de modelo) para inspección rápida de conteos;
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
- preservación de líneas no soportadas como ítems `Text` en OptiStruct.
- generación de `optistruct_file_summaries` por archivo.
- generación de `optistruct_model_summaries` por modelo.
- check de cobertura de líneas no vacías (`line_coverage_ok`) por archivo y modelo.
- ignorar `.femreader-cache` como entrada de modelo.

`src/optistruct.rs` incluye tests unitarios del parser de tarjetas y comentarios.
`src/highlighting.rs` incluye tests unitarios de lectura XML y clasificación (`keyword`, `operator`, `comment`).
`src/bin/femreader_gui.rs` incluye tests unitarios de alineación de cabecera visual y mapeo de color de fondo del highlighting.
`src/bin/femreader_gui.rs` también valida normalización de rutas de Windows y color por defecto (negro) para texto no tokenizado con highlighting activo.

Ejecutar:

```bash
cargo test
```

## 8. Extensiones recomendadas

- extraer factoría compartida para construir el `ReaderPipeline` (evitar duplicación entre CLI/GUI/tests),
- soportar parsing FEM más rico (`Node`, `Element`, `Material`, etc.),
- incorporar visualización de grafo de dependencias en la GUI.
