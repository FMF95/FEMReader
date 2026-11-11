# Integración de FEMReader como DLL para Python

Esta guía muestra cómo cargar `femreader.dll` desde Python y obtener el mismo JSON que devuelve el CLI, usando `ctypes`.

## 1) API FFI disponible

La DLL exporta estas funciones:

- `femreader_read_models(input_dir, pretty, delete_cache) -> FemreaderResult`
- `femreader_free_string(ptr)`

La estructura `FemreaderResult` contiene:

- `json`: puntero UTF-8 con el resultado en caso de éxito.
- `error`: puntero UTF-8 con el error en caso de fallo.

Regla de uso:

- Si `json != NULL`, la llamada fue exitosa.
- Si `error != NULL`, la llamada falló.

## 2) Compilar la DLL

En la raíz del proyecto:

```bash
cargo build --release --lib
```

Artefacto principal en Windows:

- `target\release\femreader.dll`

## 3) Ubicación de la DLL para Python

Opciones comunes:

1. Copiar `femreader.dll` junto a tu script Python.
2. Mantenerla en `target\release\` y pasar la ruta absoluta al cargarla.

## 4) Ejemplo completo con ctypes

```python
import ctypes
from ctypes import Structure, c_char_p, c_uint8
from pathlib import Path


class FemreaderResult(Structure):
    _fields_ = [
        ("json", c_char_p),
        ("error", c_char_p),
    ]


class FemReaderClient:
    def __init__(self, dll_path: str):
        self._dll = ctypes.CDLL(dll_path)

        # Firma: femreader_read_models(const char* input_dir, uint8 pretty, uint8 delete_cache)
        self._dll.femreader_read_models.argtypes = [c_char_p, c_uint8, c_uint8]
        self._dll.femreader_read_models.restype = FemreaderResult

        # Firma: femreader_free_string(char* ptr)
        self._dll.femreader_free_string.argtypes = [c_char_p]
        self._dll.femreader_free_string.restype = None

    def read_models(self, input_dir: str, pretty: bool = True, delete_cache: bool = False) -> str:
        abs_input = str(Path(input_dir).resolve())
        result = self._dll.femreader_read_models(
            abs_input.encode("utf-8"),
            1 if pretty else 0,
            1 if delete_cache else 0,
        )

        try:
            if result.error:
                message = ctypes.cast(result.error, c_char_p).value
                raise RuntimeError((message or b"Unknown FEMReader error").decode("utf-8", errors="replace"))

            if not result.json:
                raise RuntimeError("FEMReader returned no output.")

            json_text = ctypes.cast(result.json, c_char_p).value
            if json_text is None:
                raise RuntimeError("Invalid UTF-8 output from FEMReader.")

            return json_text.decode("utf-8", errors="strict")

        finally:
            # Importante: liberar ambos punteros cuando existan.
            if result.json:
                self._dll.femreader_free_string(result.json)
            if result.error:
                self._dll.femreader_free_string(result.error)


if __name__ == "__main__":
    # Ajusta la ruta según tu entorno.
    dll = r"D:\Programación\FEMReader\target\release\femreader.dll"
    input_models_dir = r"D:\ruta\absoluta\a\modelos"

    client = FemReaderClient(dll)
    output_json = client.read_models(input_models_dir, pretty=True, delete_cache=False)

    print(output_json)
```

## 5) Uso rápido

```bash
python tu_script.py
```

## 6) Parsear el JSON en Python

Si quieres trabajar con objetos Python:

```python
import json

data = json.loads(output_json)
print(type(data), len(data) if isinstance(data, list) else "obj")
```

## 7) Flujo recomendado

1. Compila FEMReader en `release`.
2. Carga `femreader.dll` con `ctypes.CDLL`.
3. Llama `femreader_read_models(ruta_absoluta, 1, 0)`.
4. Decodifica `result.json` como UTF-8.
5. Libera siempre memoria con `femreader_free_string`.

## 8) Notas importantes

- `input_dir` debe ser ruta absoluta y existente.
- `pretty = 1` activa JSON indentado (igual que `--pretty` en CLI).
- `delete_cache = 1` elimina `.femreader-cache` al finalizar.
- La lógica de lectura es la misma para CLI, .NET y Python (misma tubería interna).

## 9) Solución de problemas

- Error `OSError: [WinError 126]`: Python no encuentra dependencias de la DLL.
  - Verifica que `femreader.dll` esté en una ruta accesible.
  - Prueba ejecutar desde la carpeta donde está la DLL.
- Resultado vacío:
  - Verifica que `input_dir` sea absoluto y exista.
- Excepción con texto de FEMReader:
  - Revisa el contenido de `error`; es el mensaje nativo devuelto por Rust.
