# OptiStruct Cards Guide

Esta guía describe cómo extender el lector de tarjetas de OptiStruct en FEMReader.

## 1. Estructura actual

El parser está en `src/optistruct.rs` y produce `optistruct_items` en `ModelContent`.

Cada item es:

- `Comment`: comentario `$...` con clasificación (`Comment` o `HyperMesh Comment`)
- `Text`: línea no comentario sin parser de tarjeta específico (se conserva tal cual)
- `Card`: tarjeta de OptiStruct con:
  - `id` único del ítem
  - `location` (`file_id`, `file_path`, `start_line`, `end_line`, `ordinal`)
  - `location.file_name`
  - `text` (línea(s) original(es) sin reescritura)
  - `card_name`
  - `long_format` (16 chars/campo) o formato corto (8 chars/campo)
  - `fields` (campos crudos, incluyendo `F1` como tipo de tarjeta)
  - `attributes` (campos tipados de la tarjeta en JSON)

## 2. Tarjetas ya implementadas

| Tarjeta | Tipo Rust | Campos principales |
|---|---|---|
| `GRID` | `GridCard` | `id`, `cp`, `x1`, `x2`, `x3`, `cd`, `ps`, `seid` |
| `CQUAD4` | `Cquad4Card` | `id`, `pid`, `g1..g4`, `theta_or_mcid`, `zoffs`, `tflag`, `t1..t4` |
| `CTRIA3` | `Ctria3Card` | `id`, `pid`, `g1..g3`, `theta_or_mcid`, `zoffs`, `tflag`, `t1..t3` |
| `FORCE` | `ForceCard` | `sid`, `g`, `cid`, `f`, `n1`, `n2`, `n3` |
| `MOMENT` | `MomentCard` | `sid`, `g`, `cid`, `m`, `n1`, `n2`, `n3` |
| `CBUSH` | `CbushCard` | `eid`, `pid`, `ga`, `gb`, `cid`, `s`, `ocid`, `s1..s3` |
| `PBUSH` | `PbushCard` | `pid`, `k1..k6`, `b1..b6`, `ge1..ge6`, `sa`, `sb` |
| `PSHELL` | `PshellCard` | `pid`, `mid1`, `t`, `mid2`, `twelve_i_over_t3`, `mid3`, `ts_over_t`, `nsm`, `z1`, `z2`, `mid4` |
| `MAT1` | `Mat1Card` | `mid`, `e`, `g`, `nu`, `rho`, `a`, `tref`, `ge`, `st`, `sc`, `ss`, `mcsid` |
| `INCLUDE` | `IncludeCard` | `target` |
| `BEGIN BULK` | `BeginBulkCard` | marcador de inicio del bloque bulk |
| `ENDDATA` | `EndDataCard` | marcador de cierre del bloque de datos |

Solo se parsean por campos las tarjetas con parser específico. Las líneas no soportadas se conservan como `Text`.
Las tarjetas añadidas mantienen `type: "Card"` y exponen sus campos tipados en `attributes`; no se añade una colección ni cambia la estructura de primer nivel del JSON.

Notas de campos:

- En formato corto, se preservan hasta `F1..F10` por línea de tarjeta.
- `F1` corresponde al primer campo (tipo de tarjeta), por ejemplo `GRID`.
- Excepción: `BEGIN BULK` y `ENDDATA` son tarjetas sin campos (`fields: []`).

Comentarios HyperMesh detectados automáticamente:

- `$HMNAME`
- `$HWCOLOR`
- `$HMMOVE`
- `$HMDPRP`

Campos parseados por tipo:

- `HMNAME entity_type, id, name, id_pool`
- `HWCOLOR entity_type, id, color_id`
- `HMMOVE collector_id, raw_fields (incluye continuaciones)`
- `HMDPRP elements, ids` (si no hay keyword de elementos, todos los campos se interpretan como `ids`)

Casos cubiertos:

- `HMNAME` con `id` pegado al nombre entre comillas (ej. `1"COMP_A"`).
- `HMMOVE` con continuidad en línea de comentario siguiente indentada:
  - `$HMMOVE        1`
  - `$              1THRU         100`
- `HMDPRP` con continuidad en líneas `$` indentadas (campos de 8 caracteres).

## 3. Cómo añadir una tarjeta nueva

1. Añade un struct en `src/model.rs` para la tarjeta (campos tipados con `Option<T>`).
2. Añade la variante en `OptiStructCardKind`.
3. En `src/optistruct.rs`, extiende `parse_card_kind` con el nombre de la tarjeta y mapeo de campos.
4. Extiende serialización en `src/serializer.rs`:
   - `serialize_optistruct_card_attributes`
   - función específica de serialización y su entrada en `card_field_templates`.
5. Extiende visualización en GUI (`src/bin/femreader_gui.rs`) en `render_card_named_fields`.
6. Añade tests unitarios en `src/optistruct.rs` y, si aplica, un test de integración en `tests/integration_reader.rs`.

## 4. Convenciones recomendadas

- Mantener `fields` con orden original para depuración.
- Usar `Option<T>` en campos numéricos para evitar perder la tarjeta si un campo es inválido.
- No eliminar comentarios: conservarlos como `Comment` con su línea original.
- No forzar parseo de líneas no soportadas: mantenerlas como `Text`.
- Si el comentario coincide con prefijos HyperMesh, marcarlo como `HyperMesh Comment`.
- Evitar mezclar reglas de solver en el parser: la selección de adapter vive en `dependency.rs`.

## 5. Notas de formato OptiStruct soportadas

- Formato corto: hasta 10 campos por línea, 8 caracteres por campo.
- Formato largo: 16 caracteres por campo (`CARD*` en primera columna).
- Continuación: línea que comienza por `*`.
- Comentarios: `$` (línea completa o inline).

## 6. Resumen por archivo

Además de `optistruct_items`, se genera `optistruct_file_summaries` con:

- check de cobertura de líneas (`total_non_empty_lines`, `covered_non_empty_lines`, `line_coverage_ok`)
- totales por archivo (`total_items`, `card_items`, `comment_items`, `hypermesh_comment_items`, `text_items`)
- conteo por tarjeta (`card_counts`)

Esto permite construir resúmenes por modelo (por ejemplo, total de `CQUAD4` o total de comentarios) sumando los archivos de ese modelo.

También se expone `optistruct_model_summaries` con los mismos conteos agregados por modelo.
