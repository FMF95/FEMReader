# Integración de FEMReader (Rust) con una app .NET

Esta guía explica cómo compilar FEMReader para producción, cómo invocarlo desde .NET y qué parámetros CLI existen actualmente.

> Si necesitas integración **como DLL** con P/Invoke, consulta `DOTNET_DLL_INTEGRATION.md`.

## 1) Estrategia recomendada de integración

La forma más simple y robusta es usar FEMReader como **proceso externo** (`femreader.exe`) desde .NET.

Ventajas:

- Integración rápida (sin FFI/PInvoke).
- Aislamiento de errores y dependencias.
- Actualización sencilla de la herramienta Rust sin recompilar toda la app .NET.

## 2) Compilación para uso desde .NET

Compilar en modo release:

```bash
cargo build --release
```

Binario generado:

- `target\release\femreader.exe`

Si también quieres distribuir GUI:

```bash
cargo build --release --features gui --bin femreader_gui
```

## 3) Dónde colocar el ejecutable

Opciones típicas:

1. Copiar `femreader.exe` junto al `.exe` de tu app .NET.
2. Guardarlo en una carpeta interna, por ejemplo `tools\femreader\femreader.exe`.

Recomendación: fijar una ruta conocida y versionar el binario dentro del paquete/instalador de tu app.

## 4) Formas de llamada desde .NET

## 4.1 Generar JSON en archivo (recomendado para payload grande)

```csharp
using System.Diagnostics;

var femreaderPath = @"C:\MiApp\tools\femreader\femreader.exe";
var inputDir = @"C:\Modelos\Caso_01";
var outputJson = @"C:\Temp\femreader-output.json";

var psi = new ProcessStartInfo
{
    FileName = femreaderPath,
    Arguments = $"\"{inputDir}\" --output \"{outputJson}\"",
    UseShellExecute = false,
    RedirectStandardOutput = true,
    RedirectStandardError = true,
    CreateNoWindow = true
};

using var process = Process.Start(psi)!;
var stderr = await process.StandardError.ReadToEndAsync();
await process.WaitForExitAsync();

if (process.ExitCode != 0)
{
    throw new Exception($"FEMReader falló ({process.ExitCode}): {stderr}");
}

// Leer outputJson desde .NET (System.Text.Json, Newtonsoft, etc.)
```

## 4.2 Recibir JSON por stdout

```csharp
using System.Diagnostics;

var psi = new ProcessStartInfo
{
    FileName = femreaderPath,
    Arguments = $"\"{inputDir}\"",
    UseShellExecute = false,
    RedirectStandardOutput = true,
    RedirectStandardError = true,
    CreateNoWindow = true
};

using var process = Process.Start(psi)!;
var stdout = await process.StandardOutput.ReadToEndAsync(); // JSON
var stderr = await process.StandardError.ReadToEndAsync();
await process.WaitForExitAsync();

if (process.ExitCode != 0)
{
    throw new Exception($"FEMReader falló ({process.ExitCode}): {stderr}");
}
```

## 5) Parámetros CLI implementados (actuales)

Comando base:

```text
femreader <input_directory> [--output <output_file>] [--pretty] [--delete-cache]
```

| Parámetro | Tipo | Obligatorio | Descripción |
|---|---|---|---|
| `<input_directory>` | posicional | Sí | Directorio raíz de modelos a escanear. |
| `--output <output_file>` | opción con valor | No | Guarda el JSON en archivo en lugar de imprimirlo por stdout. |
| `--pretty` | flag | No | Serializa JSON con formato legible (indentado). |
| `--delete-cache` | flag | No | Elimina `.femreader-cache` al finalizar la lectura. |
| `-h`, `--help` | flag | No | Muestra ayuda y termina sin procesar. |

Notas:

- Si `--output` no se usa, el JSON sale por `stdout`.
- Si `--delete-cache` no se usa, la cache se conserva (útil para ejecuciones repetidas).
- Cualquier parámetro no reconocido devuelve error.

## 6) Parámetro de entrada en GUI

GUI (`femreader_gui`) acepta opcionalmente:

- `<input_directory>` como primer argumento.

Ejemplo:

```bash
femreader_gui.exe "C:\Modelos\Caso_01"
```

## 7) Recomendaciones de rendimiento

Para minimizar latencia tras la llamada desde .NET:

1. Usa siempre `target\release\femreader.exe` (no debug).
2. Para consumo máquina-a-máquina, evita `--pretty`.
3. Si ejecutas muchas veces sobre el mismo árbol, **no** uses `--delete-cache`.
4. Para JSON grande, prefiere `--output` a archivo.
5. Reutiliza rutas absolutas y evita validaciones redundantes en cada llamada.

## 8) Patrón de uso recomendado para tu caso (.NET + JSON estructurado)

1. Tu app .NET llama `femreader.exe` con `<input_directory> --output <archivo_json>`.
2. FEMReader termina y devuelve `exit code`.
3. .NET valida `exit code == 0`.
4. .NET deserializa el JSON y lo carga en su modelo de dominio.

Este flujo es el más simple, estable y mantenible para integrar Rust + .NET sin complejidad adicional.
