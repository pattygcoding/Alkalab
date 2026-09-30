<#
.SYNOPSIS
    Builds the Alkalab WebAssembly package into web/pkg (Windows convenience wrapper).

.DESCRIPTION
    Thin wrapper around scripts/build.mjs, which is the single source of truth
    (CI runs it directly on Linux). It compiles the crate with wasm-pack, renames
    the generated binary to web/pkg/alkalab.wasm, points the generated glue at
    the new name and prints the resulting file sizes.

.EXAMPLE
    npm run build
    powershell -File scripts/build.ps1 --no-opt
#>
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    & node scripts/build.mjs @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
