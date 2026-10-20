param(
    [string]$Executable = "$PSScriptRoot\..\target\release\orbit.exe",
    [int]$WarmupSeconds = 5,
    [int]$SampleSeconds = 10,
    [string]$Output = "$PSScriptRoot\..\artifacts\footprint.json"
)
$ErrorActionPreference='Stop'
python (Join-Path $PSScriptRoot 'measure-desktop.py') $Executable --warmup $WarmupSeconds --seconds $SampleSeconds --output $Output
if ($LASTEXITCODE -ne 0) { throw 'Orbit/WebView2 process-tree measurement failed.' }
