# Build the Nim engine to wasm and generate the web glue.
#   .\build.ps1
$ErrorActionPreference = 'Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
  node build.mjs
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
