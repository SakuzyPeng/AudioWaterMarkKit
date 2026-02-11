# WinUI Publish Size Experiments

## Scope

Platform: `win-x64`  
Project: `winui-app/AWMKit/AWMKit.csproj`  
Build machine: `win-pc`  
Rust native lib: `cargo build --release --features ffi,app,bundled,multichannel`

## Commands Used

All runs used `--self-contained true` and `-p:Platform=x64`.

1. Baseline single-file:

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj `
  -c Release -r win-x64 `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:IncludeAllContentForSelfExtract=true `
  -p:EnableCompressionInSingleFile=false `
  -p:PublishTrimmed=false `
  -p:PublishAot=false
```

2. Baseline + compression:

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj `
  -c Release -r win-x64 `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:IncludeAllContentForSelfExtract=true `
  -p:EnableCompressionInSingleFile=true `
  -p:PublishTrimmed=false `
  -p:PublishAot=false
```

3. Trim (`partial`) no compression:

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj `
  -c Release -r win-x64 `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:IncludeAllContentForSelfExtract=true `
  -p:EnableCompressionInSingleFile=false `
  -p:PublishTrimmed=true `
  -p:TrimMode=partial `
  -p:PublishAot=false
```

4. Trim (`partial`) + compression:

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj `
  -c Release -r win-x64 `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:IncludeAllContentForSelfExtract=true `
  -p:EnableCompressionInSingleFile=true `
  -p:PublishTrimmed=true `
  -p:TrimMode=partial `
  -p:PublishAot=false
```

5. AOT + full trim:

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj `
  -c Release -r win-x64 `
  -p:PublishSingleFile=true `
  -p:PublishAot=true `
  -p:PublishTrimmed=true `
  -p:TrimMode=full
```

## Result Table

| Mode | AWMKit.exe | Publish total | File count |
|---|---:|---:|---:|
| baseline | 197.85 MB | 198.09 MB | 2 |
| compress | 87.70 MB | 87.94 MB | 2 |
| trim | 102.62 MB | 102.86 MB | 2 |
| trim + compress | **52.45 MB** | **52.69 MB** | 2 |
| aot | 8.59 MB | 136.51 MB | 291 |
| aot + compress | 8.59 MB | 136.51 MB | 291 |

## Decision

Current best release profile for WinUI is:

- `PublishSingleFile=true`
- `PublishTrimmed=true`
- `TrimMode=partial`
- `EnableCompressionInSingleFile=true`
- `PublishAot=false`

Reason:

- Lowest practical distribution size.
- Keeps single-file extraction behavior (external DB + bundled runtime behavior unchanged).
- Avoids current AOT runtime/UX regression in key summary panel.

## Known Issue (AOT Path)

Under NativeAOT test builds, key summary panel behavior is currently unstable (slot summary does not refresh/display correctly in UI).  
Until this is fully root-caused and fixed, AOT is not selected as default release path.
