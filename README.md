# FEMReader

Lector de modelos FEM en Rust, orientado a escaneo rápido, resolución de dependencias y salida consistente para integración con otras herramientas.

## Funcionalidades principales

- **Core library** para escaneo de archivos, resolución de referencias y ensamblado de modelos.
- **CLI** para procesar un directorio y exportar JSON versionado.
- **Cache incremental oculto** (`.femreader-cache`) para evitar reprocesar archivos no modificados.
- **GUI de escritorio** para visualizar modelos, archivos y dependencias.
- **Clasificación de archivos**: `Model` (participa en dependencias/modelos) y `NonModel` (se escanea pero no se procesa en el grafo FEM).
- **Lectura inicial de tarjetas OptiStruct** con objetos tipados para `GRID`, `CQUAD4`, `CTRIA3`, `FORCE`, `MOMENT` e `INCLUDE`.
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
- Ver la estructura visual de lectura OptiStruct (por archivo, línea, comentario y tarjeta).
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
- `models`

## Desarrollo y arquitectura

Consulta la guía técnica para desarrolladores:

- `DEVELOPER_GUIDE.md`
- `OPTISTRUCT_CARDS_GUIDE.md` (cómo añadir nuevas tarjetas al parser de OptiStruct)

## Tests

```bash
cargo test
```
