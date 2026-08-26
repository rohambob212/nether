# Fetch the Aether sources Nether links against (Windows).
$ErrorActionPreference = "Stop"

$dest = Join-Path $PSScriptRoot "..\third_party\Aether"

if ((Test-Path (Join-Path $dest "aether")) -and (Test-Path (Join-Path $dest "quiche"))) {
    Write-Host "[nether] $dest already present, skipping clone."
    exit 0
}

New-Item -ItemType Directory -Force -Path (Join-Path $PSScriptRoot "..\third_party") | Out-Null
Write-Host "[nether] cloning Aether into $dest ..."
git clone --depth 1 https://github.com/CluvexStudio/Aether $dest
Write-Host "[nether] done."
