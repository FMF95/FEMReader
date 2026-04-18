# OptiStruct Cards Guide

Esta guía describe cómo extender el lector de tarjetas de OptiStruct en FEMReader.

## 1. Estructura actual

El parser está en `src/optistruct.rs` y produce `optistruct_items` en `ModelContent`.

Cada item es:

- `Comment`: comentario `$...`
- `Card`: tarjeta de OptiStruct con:
  - `location` (`file_id`, `file_path`, `start_line`, `end_line`, `ordinal`)
  - `card_name`
  - `long_format` (16 chars/campo) o formato corto (8 chars/campo)
  - `fields` (campos crudos)
  - `kind` (tarjeta tipada)

## 2. Tarjetas ya implementadas

| Tarjeta | Tipo Rust | Campos principales |
|---|---|---|
| `GRID` | `GridCard` | `id`, `cp`, `x1`, `x2`, `x3`, `cd`, `ps`, `seid` |
| `CQUAD4` | `Cquad4Card` | `id`, `pid`, `g1..g4`, `theta_or_mcid`, `zoffs`, `tflag`, `t1..t4` |
| `CTRIA3` | `Ctria3Card` | `id`, `pid`, `g1..g3`, `theta_or_mcid`, `zoffs`, `tflag`, `t1..t3` |
| `FORCE` | `ForceCard` | `sid`, `g`, `cid`, `f`, `n1`, `n2`, `n3` |
| `MOMENT` | `MomentCard` | `sid`, `g`, `cid`, `m`, `n1`, `n2`, `n3` |
| `INCLUDE` | `IncludeCard` | `target` |

Las tarjetas no tipadas se guardan como `OptiStructCardKind::Unknown`.

## 3. Cómo añadir una tarjeta nueva

1. Añade un struct en `src/model.rs` para la tarjeta (campos tipados con `Option<T>`).
2. Añade la variante en `OptiStructCardKind`.
3. En `src/optistruct.rs`, extiende `parse_card_kind` con el nombre de la tarjeta y mapeo de campos.
4. Extiende serialización en `src/serializer.rs`:
   - `serialize_optistruct_kind_label`
   - `serialize_optistruct_card_attributes`
   - función específica de serialización.
5. Extiende visualización en GUI (`src/bin/femreader_gui.rs`) en `card_kind_summary`.
6. Añade tests unitarios en `src/optistruct.rs` y, si aplica, un test de integración en `tests/integration_reader.rs`.

## 4. Convenciones recomendadas

- Mantener `fields` con orden original para depuración.
- Usar `Option<T>` en campos numéricos para evitar perder la tarjeta si un campo es inválido.
- No eliminar comentarios: conservarlos como `Comment` con su línea original.
- Evitar mezclar reglas de solver en el parser: la selección de adapter vive en `dependency.rs`.

## 5. Notas de formato OptiStruct soportadas

- Formato corto: hasta 10 campos por línea, 8 caracteres por campo.
- Formato largo: 16 caracteres por campo (`CARD*` en primera columna).
- Continuación: línea que comienza por `*`.
- Comentarios: `$` (línea completa o inline).
