# Integración de FEMReader como DLL para .NET

Esta guía muestra cómo compilar FEMReader como DLL y cómo llamarlo desde .NET para obtener el mismo JSON que devuelve el CLI.

## 1) Qué se implementó

Se añadió soporte `cdylib` y API FFI con estas funciones exportadas:

- `femreader_read_models(input_dir, pretty, delete_cache) -> FemreaderResult`
- `femreader_free_string(ptr)`

`FemreaderResult` devuelve:

- `json`: puntero UTF-8 con el resultado en caso de éxito.
- `error`: puntero UTF-8 con el error en caso de fallo.

> Si `json != IntPtr.Zero`, la llamada fue exitosa.  
> Si `error != IntPtr.Zero`, la llamada falló.

## 2) Compilar la DLL

En la raíz del proyecto (solo DLL):

```bash
cargo build --release --lib
```

Si además quieres el ejecutable CLI en release:

```bash
cargo build --release --bin femreader
```

Artefactos relevantes en Windows:

- `target\release\femreader.dll`
- `target\release\femreader.dll.lib` (import library para toolchains nativas)

## Publicación automática en GitHub Releases

El workflow `.github/workflows/release.yml` compila la DLL con Rust estable en `windows-latest` y adjunta `target\release\femreader.dll` a un Release de GitHub.

Para lanzar una versión nueva:

```bash
git add .
git commit -m "Prepare release v1.0.0"
git tag v1.0.0
git push origin main
git push origin v1.0.0
```

Al subir el tag `v1.0.0`, GitHub Actions crea o actualiza el Release correspondiente. Sustituye `v1.0.0` por el nombre de la versión que corresponda y adapta el nombre de la rama si no usas `main`.

## 3) Verificar que el CLI sigue funcionando

La llamada CLI que pediste sigue vigente:

```bash
cargo run -- /ruta/absoluta/a/modelos --pretty
```

También puedes usar:

```bash
cargo run -- /ruta/absoluta/a/modelos --output /ruta/absoluta/salida.json --pretty
```

## 4) Ejemplo de uso desde .NET (P/Invoke)

```csharp
using System;
using System.Runtime.InteropServices;

internal static class FemReaderNative
{
    [StructLayout(LayoutKind.Sequential)]
    internal struct FemreaderResult
    {
        public IntPtr json;
        public IntPtr error;
    }

    [DllImport("femreader", CallingConvention = CallingConvention.Cdecl)]
    internal static extern FemreaderResult femreader_read_models(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string inputDir,
        byte pretty,
        byte deleteCache);

    [DllImport("femreader", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void femreader_free_string(IntPtr ptr);
}

public static class FemReaderClient
{
    public static string ReadModels(string absoluteInputDir, bool pretty = true, bool deleteCache = false)
    {
        var result = FemReaderNative.femreader_read_models(
            absoluteInputDir,
            pretty ? (byte)1 : (byte)0,
            deleteCache ? (byte)1 : (byte)0);

        try
        {
            if (result.error != IntPtr.Zero)
            {
                var message = Marshal.PtrToStringUTF8(result.error) ?? "Unknown FEMReader error";
                throw new InvalidOperationException(message);
            }

            if (result.json == IntPtr.Zero)
            {
                throw new InvalidOperationException("FEMReader returned no output.");
            }

            return Marshal.PtrToStringUTF8(result.json)
                   ?? throw new InvalidOperationException("Invalid UTF-8 output from FEMReader.");
        }
        finally
        {
            if (result.json != IntPtr.Zero)
            {
                FemReaderNative.femreader_free_string(result.json);
            }

            if (result.error != IntPtr.Zero)
            {
                FemReaderNative.femreader_free_string(result.error);
            }
        }
    }
}
```

## 5) Dónde ubicar la DLL

Opciones comunes:

1. Junto al `.exe` de tu app .NET.
2. En una carpeta de runtime (`runtimes/win-x64/native/`) si empaquetas como NuGet.

.NET debe poder resolver `femreader.dll` en tiempo de ejecución.

## 6) Flujo recomendado para tu caso

1. Compila FEMReader en `release`.
2. Desde .NET llama `femreader_read_models("ruta_absoluta", 1, 0)`.
3. Recibe `json` y deserialízalo en tu modelo C#.
4. Libera siempre memoria con `femreader_free_string`.

## 7) Notas importantes

- `input_dir` debe ser ruta absoluta y existente.
- `pretty = 1` activa JSON indentado (igual que `--pretty` en CLI).
- `delete_cache = 1` elimina `.femreader-cache` al finalizar.
- La lógica de lectura es la misma para CLI y DLL (misma tubería interna).
