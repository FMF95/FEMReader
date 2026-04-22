# FEMReader

Lector de modelos FEM en Rust, orientado a escaneo rápido, resolución de dependencias y salida consistente para integración con otras herramientas.

## Funcionalidades principales

- **Core library** para escaneo de archivos, resolución de referencias y ensamblado de modelos.
- **CLI** para procesar un directorio y exportar JSON versionado.
- **Cache incremental oculto** (`.femreader-cache`) para evitar reprocesar archivos no modificados (se ignora como entrada de modelo).
- **GUI de escritorio** para visualizar modelos, archivos y dependencias.
- **Clasificación de archivos**: `Model` (participa en dependencias/modelos) y `NonModel` (se escanea pero no se procesa en el grafo FEM).
- **Lectura de tarjetas OptiStruct** con objetos tipados para `GRID`, `CQUAD4`, `CTRIA3`, `FORCE`, `MOMENT`, `INCLUDE`, `BEGIN BULK` y `ENDDATA`.
- **Parsing estricto por tarjeta soportada**: solo las tarjetas con parser específico se interpretan por campos (8/16). Líneas no soportadas y no comentario se conservan como `Text`.
- **Campos crudos con F1 incluido**: en tarjetas, `F1` conserva el primer campo de 8 caracteres (tipo de tarjeta) y el resto se desplaza (`F2..`).
- **Tarjetas delimitadoras sin campos**: `BEGIN BULK` y `ENDDATA` se identifican como tarjetas propias con `fields: []`.
- **Comentarios OptiStruct clasificados** como `Comment` o `HyperMesh Comment` (`$HMNAME`, `$HWCOLOR`, `$HMMOVE`, `$HMDPRP`).
- **Resúmenes OptiStruct por archivo y por modelo**: totales de tarjetas/comentarios/texto + conteo por tarjeta, con check de cobertura de líneas no vacías.
- **IDs únicos determinísticos** para entidades del modelo (directorios, archivos, dependencias, modelos e ítems OptiStruct), en formato hash estable sin prefijos.
- **Campos de nombre legible**: `name` para archivos/directorios y `file_name` para ubicaciones de ítems OptiStruct.
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

Eliminar la cache después de leer (modo efímero):

```bash
cargo run -- /ruta/absoluta/a/modelos --delete-cache
```

### Uso de `.femreader-cache`

- Guarda por archivo el `path`, fecha de modificación y tamaño para detectar cambios incrementales.
- Se crea en el directorio raíz escaneado y se marca como oculto cuando el sistema operativo lo soporta.
- Si no quieres persistencia, usa `--delete-cache` para eliminarla al terminar la lectura.
- El scanner ignora `.femreader-cache` para que no aparezca como archivo de modelo.

## Uso de la GUI

Lanzar la interfaz gráfica:

```bash
cargo run --features gui --bin femreader_gui -- /ruta/absoluta/a/modelos
```

En la GUI puedes:

- Cargar un directorio de modelos.
- Ver resumen de directorios, archivos de modelo, archivos no modelo, dependencias y referencias sin resolver.
- Seleccionar cada modelo y revisar sus archivos, solver detectado y relaciones de dependencia.
- Ver el listado de archivos `NonModel` (escaneados pero fuera del grafo de modelos FEM).
- Ver la estructura visual de lectura OptiStruct con encabezados `L# [Tipo] <línea original>`.
- Ver en tarjetas los campos por nombre (`SID`, `PID`, `X1`, etc.) en lugar de etiquetas `F1/F2/...`.
- Ver un resumen OptiStruct por archivo y totales de resumen por modelo (incluyendo check de cobertura de líneas).
- Activar la opción para eliminar `.femreader-cache` tras cada carga.

## Contrato de salida JSON

La salida serializada incluye:

- `schema_version`
- `root_directory`
- `directories`
- `files`
- `dependencies`
- `unresolved_references`
- `optistruct_items`
- `optistruct_file_summaries`
- `optistruct_model_summaries`
- `models`

Notas:

- `id` y `file_id` contienen IDs únicos (no rutas).
- `path` y `file_path` mantienen la ruta completa.
- Los IDs únicos se serializan como hash estable (sin prefijos tipo `file_`, `model_`, etc.).
- En `optistruct_items`, los ítems no parseados como tarjeta soportada se serializan como tipo `Text`.
- En `Card`, la línea original se serializa en `text` (igual criterio que `Comment`/`Text`).
- JSON no admite comentarios nativos; se añade `json_guide` como ayuda de lectura de secciones/plantillas de campos.
- `schema_version` se toma desde una constante única (`model::SCHEMA_VERSION`) y se valida con tests de guardia de contrato.

## Desarrollo y arquitectura

Consulta la guía técnica para desarrolladores:

- `DEVELOPER_GUIDE.md`
- `OPTISTRUCT_CARDS_GUIDE.md` (cómo añadir nuevas tarjetas al parser de OptiStruct)

## Tests

```bash
cargo test
```
