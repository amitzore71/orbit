param([string]$InnoCompiler = '', [switch]$SkipInstaller)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $projectRoot
try {
    cargo test --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    python scripts\write-build-info.py
    if ($LASTEXITCODE -ne 0) { throw 'Build metadata failed.' }
    $stagingRoot = Join-Path $projectRoot 'dist\Orbit-0.1.0-windows-x64'
    New-Item -ItemType Directory -Path $stagingRoot -Force | Out-Null
    Copy-Item -LiteralPath 'target\release\orbit.exe' -Destination $stagingRoot
    Copy-Item -LiteralPath 'README.md','LICENSE' -Destination $stagingRoot
    $stagingDocs = Join-Path $stagingRoot 'docs'
    New-Item -ItemType Directory -Path $stagingDocs -Force | Out-Null
    Copy-Item -LiteralPath 'docs\PERFORMANCE.md','docs\RELEASE-NOTES.md','docs\VALIDATION.md','docs\STABILITY.md','docs\stability-results.json','docs\ui-optimization.json' -Destination $stagingDocs
    Copy-Item -LiteralPath 'docs\build-info.json' -Destination $stagingDocs
    if (Test-Path -LiteralPath 'docs\branding-validation.json') { Copy-Item -LiteralPath 'docs\branding-validation.json' -Destination $stagingDocs }
    if (Test-Path -LiteralPath 'docs\footprint.json') { Copy-Item -LiteralPath 'docs\footprint.json' -Destination $stagingDocs }
    python scripts\license-notices.py (Join-Path $stagingRoot 'THIRD-PARTY-NOTICES.txt')
    if ($LASTEXITCODE -ne 0) { throw 'Could not assemble license notices.' }
    Compress-Archive -Path (Join-Path $stagingRoot '*') -DestinationPath 'dist\Orbit-0.1.0-windows-x64.zip' -Force
    if (-not $SkipInstaller) {
        if (-not $InnoCompiler) {
            $candidates = @('tools\inno\ISCC.exe', 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe', 'C:\Program Files\Inno Setup 7\ISCC.exe')
            $InnoCompiler = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        }
        if (-not $InnoCompiler) { throw 'ISCC.exe was not found. Pass -InnoCompiler, or use -SkipInstaller to build only the portable ZIP.' }
        & $InnoCompiler packaging\orbit.iss
        if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    }
    Get-ChildItem dist -File | Select-Object Name,Length
}
finally { Pop-Location }
