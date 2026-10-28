# Empaqueta ONNX Runtime dentro de la app en Windows.
# Uso: .\scripts\bundle-onnx.ps1
# Resultado: src-tauri\resources\onnxruntime.dll
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Res = Join-Path $Root "src-tauri\resources"
$Tmp = Join-Path $env:TEMP ("pluma-onnx-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $Tmp | Out-Null

$Url = "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-win-x64-1.28.0.zip"
$Sha = "abef733dacbe2f571547a7150b479b5cb9cc0df22f96c24983a42cadb1b4f8bc"
$Zip = Join-Path $Tmp "onnx.zip"

try {
    Write-Host "Descargando ONNX Runtime 1.28…"
    Invoke-WebRequest -Uri $Url -OutFile $Zip

    $Actual = (Get-FileHash $Zip -Algorithm SHA256).Hash.ToLower()
    if ($Actual -ne $Sha) {
        throw "Hash inválido: $Actual (esperado $Sha)"
    }

    Expand-Archive -Path $Zip -DestinationPath $Tmp -Force
    $Dll = Get-ChildItem -Path $Tmp -Filter "onnxruntime.dll" -Recurse | Select-Object -First 1
    if (-not $Dll) { throw "No se encontró onnxruntime.dll en el archivo" }

    New-Item -ItemType Directory -Path $Res -Force | Out-Null
    Copy-Item $Dll.FullName (Join-Path $Res "onnxruntime.dll") -Force
    Write-Host "ONNX Runtime empaquetado en $Res\onnxruntime.dll"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
