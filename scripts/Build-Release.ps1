param([string]$InnoCompiler = '', [switch]$SkipInstaller, [switch]$SkipChecks)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $projectRoot
try {
    $version = (Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
    pnpm --dir frontend install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw 'Frontend installation failed.' }
    pnpm --dir frontend build
    if ($LASTEXITCODE -ne 0) { throw 'Static frontend build failed.' }
    if (-not $SkipChecks) {
        pnpm --dir frontend test
        if ($LASTEXITCODE -ne 0) { throw 'Frontend tests failed.' }
        cargo test --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed.' }
    }
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $stagingRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot "dist\Orbit-$version-windows-x64"))
    $distributionRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'dist'))
    if (-not $stagingRoot.StartsWith($distributionRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Staging is outside dist.' }
    if (Test-Path -LiteralPath $stagingRoot) {
        if ((Get-Item -LiteralPath $stagingRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Staging must not be a link.' }
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stagingRoot -Force | Out-Null
    Copy-Item -LiteralPath 'target\release\orbit.exe' -Destination $stagingRoot
    $stagedExecutable = Join-Path $stagingRoot 'orbit.exe'
    python scripts\check-size.py $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw 'Executable exceeds size budget.' }
    python scripts\write-build-info.py --exe $stagedExecutable
    python scripts\license-notices.py (Join-Path $stagingRoot 'THIRD-PARTY-NOTICES.txt')
    if ($LASTEXITCODE -ne 0) { throw 'Dependency notices are incomplete.' }
    if (-not $SkipChecks) {
        python scripts\branding-check.py $stagedExecutable
        if ($LASTEXITCODE -ne 0) { throw 'Legacy database check failed.' }
        python scripts\integration-check.py $stagedExecutable
        if ($LASTEXITCODE -ne 0) { throw 'Timer integration failed.' }
        python scripts\integration-failure-check.py $stagedExecutable
        if ($LASTEXITCODE -ne 0) { throw 'Persistence recovery failed.' }
        python scripts\checkpoint-recovery-check.py $stagedExecutable
        if ($LASTEXITCODE -ne 0) { throw 'Checkpoint recovery failed.' }
    }
    python scripts\measure-desktop.py $stagedExecutable --output docs\release-footprint.json
    if ($LASTEXITCODE -ne 0) { throw 'Process-tree measurement failed.' }
    python scripts\measure-window-startup.py --executable $stagedExecutable --trials 8 --output docs\release-window-startup.json
    if ($LASTEXITCODE -ne 0) { throw 'Interactive startup measurement failed.' }
    Copy-Item -LiteralPath 'README.md','LICENSE' -Destination $stagingRoot
    $stagingDocs = Join-Path $stagingRoot 'docs'
    New-Item -ItemType Directory -Path $stagingDocs -Force | Out-Null
    Copy-Item -LiteralPath 'docs\build-info.json','docs\release-footprint.json','docs\release-window-startup.json','docs\RELEASE-NOTES.md','docs\PERFORMANCE.md','docs\RELEASE-VALIDATION.md' -Destination $stagingDocs
    # Include optional extended audits only when they identify this exact executable.
    $stagedHash = (Get-FileHash -LiteralPath $stagedExecutable -Algorithm SHA256).Hash
    foreach ($auditName in @('release-steady-footprint.json','release-hidden-soak.json','release-native-performance.json','release-ui-validation.json')) {
        $auditPath = Join-Path 'docs' $auditName
        if (Test-Path -LiteralPath $auditPath) {
            $audit = Get-Content -LiteralPath $auditPath -Raw | ConvertFrom-Json
            if ($audit.executableSha256 -eq $stagedHash) { Copy-Item -LiteralPath $auditPath -Destination $stagingDocs }
        }
    }
    Compress-Archive -Path (Join-Path $stagingRoot '*') -DestinationPath "dist\Orbit-$version-windows-x64.zip" -Force
    if (-not $SkipInstaller) {
        if (-not $InnoCompiler) { $InnoCompiler = @('tools\inno\ISCC.exe','C:\Program Files (x86)\Inno Setup 6\ISCC.exe') | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1 }
        if (-not $InnoCompiler) { throw 'ISCC.exe was not found. Pass -InnoCompiler or use -SkipInstaller.' }
        & $InnoCompiler "/DAppVersion=$version" packaging\orbit.iss
        if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    }
    $verificationArgs = @('scripts\verify-release.py')
    if ($SkipInstaller) { $verificationArgs += '--skip-installer' }
    python @verificationArgs
    if ($LASTEXITCODE -ne 0) { throw 'Release verification failed.' }
    $releaseFiles = @("dist\Orbit-$version-windows-x64.zip")
    if (-not $SkipInstaller) { $releaseFiles += "dist\Orbit-$version-Setup-x64.exe" }
    $checksums = foreach ($releaseFile in $releaseFiles) { $hash=Get-FileHash -LiteralPath $releaseFile -Algorithm SHA256; $hash.Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($releaseFile) }
    [IO.File]::WriteAllLines((Join-Path $projectRoot 'dist\SHA256SUMS.txt'), [string[]]$checksums, [Text.UTF8Encoding]::new($false))
    Get-ChildItem -LiteralPath $stagingRoot -File | Select-Object Name,Length
} finally { Pop-Location }
