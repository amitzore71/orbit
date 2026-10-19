param([string]$InnoCompiler = '', [switch]$SkipInstaller, [string]$UpxPath = '')
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $projectRoot
try {
    if (-not $UpxPath) {
        $upxCommand = Get-Command upx -ErrorAction SilentlyContinue
        $UpxPath = if ($upxCommand) { $upxCommand.Source } else { Join-Path $projectRoot 'tools\upx\upx-5.2.1-win64\upx.exe' }
    }
    if (-not (Test-Path -LiteralPath $UpxPath)) { throw 'UPX 5.2.1 is required for compact releases. Install it on PATH or pass -UpxPath.' }
    $UpxPath = (Resolve-Path -LiteralPath $UpxPath).Path
    $upxVersion = & $UpxPath --version
    if ($LASTEXITCODE -ne 0 -or ($upxVersion -join ' ') -notmatch '\b5\.2\.1\b') { throw 'Use the tested UPX 5.2.1 release.' }
    cargo test --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $stagingRoot = Join-Path $projectRoot 'dist\Orbit-0.1.0-windows-x64'
    if (Test-Path -LiteralPath $stagingRoot) {
        $resolvedStage = (Resolve-Path -LiteralPath $stagingRoot).Path
        $distributionRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'dist'))
        if (-not $resolvedStage.StartsWith($distributionRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Staging directory is outside dist.' }
        if ((Get-Item -LiteralPath $resolvedStage).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Staging directory must not be a link.' }
        Remove-Item -LiteralPath $resolvedStage -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stagingRoot -Force | Out-Null
    Copy-Item -LiteralPath 'target\release\orbit.exe' -Destination $stagingRoot
    $stagedExecutable = Join-Path $stagingRoot 'orbit.exe'
    & $UpxPath --best --lzma --strip-relocs=0 --compress-resources=0 $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw 'Executable compression failed.' }
    python scripts\check-size.py $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw 'Release executable exceeds 10 MB.' }
    python scripts\write-build-info.py --exe $stagedExecutable --compression upx
    if ($LASTEXITCODE -ne 0) { throw 'Build metadata failed.' }
    python scripts\branding-check.py $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw 'Packaged executable startup or legacy-data check failed.' }
    python scripts\integration-check.py $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw 'Packaged executable timer integration check failed.' }
    python scripts\integration-failure-check.py $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw 'Packaged executable recovery check failed.' }
    Copy-Item -LiteralPath 'README.md','LICENSE' -Destination $stagingRoot
    $stagingDocs = Join-Path $stagingRoot 'docs'
    New-Item -ItemType Directory -Path $stagingDocs -Force | Out-Null
    Copy-Item -LiteralPath 'docs\PERFORMANCE.md','docs\RELEASE-NOTES.md','docs\VALIDATION.md','docs\STABILITY.md','docs\stability-results.json','docs\ui-optimization.json' -Destination $stagingDocs
    Copy-Item -LiteralPath 'docs\build-info.json' -Destination $stagingDocs
    Copy-Item -LiteralPath 'artifacts\integration-check.json','artifacts\integration-failure-check.json' -Destination $stagingDocs
    if (Test-Path -LiteralPath 'docs\branding-validation.json') { Copy-Item -LiteralPath 'docs\branding-validation.json' -Destination $stagingDocs }
    if (Test-Path -LiteralPath 'docs\footprint.json') { Copy-Item -LiteralPath 'docs\footprint.json' -Destination $stagingDocs }
    python scripts\license-notices.py (Join-Path $stagingRoot 'THIRD-PARTY-NOTICES.txt')
    if ($LASTEXITCODE -ne 0) { throw 'Could not assemble license notices.' }
    $upxLicense = Get-Content -LiteralPath 'packaging\licenses\upx--5.2.1\LICENSE' -Raw
    $upxGpl = Get-Content -LiteralPath 'packaging\licenses\upx--5.2.1\COPYING' -Raw
    $upxNotice = [Environment]::NewLine + 'UPX 5.2.1 - https://github.com/upx/upx/releases/tag/v5.2.1' + [Environment]::NewLine + $upxLicense + [Environment]::NewLine + $upxGpl
    [IO.File]::AppendAllText((Join-Path $stagingRoot 'THIRD-PARTY-NOTICES.txt'), $upxNotice, [Text.UTF8Encoding]::new($false))
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
    $verificationArgs = @('scripts\verify-release.py', '--upx', $UpxPath)
    if ($SkipInstaller) { $verificationArgs += '--skip-installer' }
    python @verificationArgs
    if ($LASTEXITCODE -ne 0) { throw 'Release verification failed.' }
    $releaseFiles = @('dist\Orbit-0.1.0-windows-x64.zip')
    if (-not $SkipInstaller) { $releaseFiles += 'dist\Orbit-0.1.0-Setup-x64.exe' }
    $checksums = foreach ($releaseFile in $releaseFiles) {
        $hash = Get-FileHash -LiteralPath $releaseFile -Algorithm SHA256
        $hash.Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($releaseFile)
    }
    [IO.File]::WriteAllLines((Join-Path $projectRoot 'dist\SHA256SUMS.txt'), [string[]]$checksums, [Text.UTF8Encoding]::new($false))
    Get-ChildItem dist -File | Select-Object Name,Length
}
finally { Pop-Location }
