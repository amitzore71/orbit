param(
    [string]$Executable = "$PSScriptRoot\..\target\release\orbit.exe",
    [int]$Cycles = 150,
    [int]$Tasks = 300
)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$verifier = Join-Path $env:WINDIR 'System32\appverif.exe'
if (-not (Test-Path -LiteralPath $verifier)) { throw 'Microsoft Application Verifier is required.' }
$testName = 'orbit-heap-check.exe'
$settingsKey = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\' + $testName
if ((Test-Path -LiteralPath $settingsKey) -and ((Get-Item -LiteralPath $settingsKey).GetValueNames().Count -gt 0)) { throw 'This test image already has verifier settings; preserve or remove them before this run.' }
$testRoot = Join-Path $projectRoot ('artifacts\verifier-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
$imagePath = Join-Path $testRoot $testName
Copy-Item -LiteralPath $Executable -Destination $imagePath
$previousLogPath = $env:VERIFIER_LOG_PATH
Push-Location $projectRoot
try {
    $env:VERIFIER_LOG_PATH = $testRoot
    Start-Process -FilePath $verifier -ArgumentList '-enable','Heaps','Handles','-for',$testName -WindowStyle Hidden -Wait
    if (-not (Test-Path -LiteralPath $settingsKey)) { throw 'Verifier settings were not enabled. Run from an elevated terminal.' }
    python scripts/stress-check.py --exe $imagePath --cycles $Cycles
    if ($LASTEXITCODE -ne 0) { throw 'The native lifecycle test failed under Application Verifier.' }
    $logs = @(Get-ChildItem -LiteralPath (Join-Path $testRoot 'AppVerifierLogs') -Filter ($testName + '*.dat') -File)
    if ($logs.Count -eq 0) { throw 'No verifier logs were created.' }
    foreach ($log in $logs) {
        $xmlPath = $log.FullName + '.xml'
        Start-Process -FilePath $verifier -ArgumentList '-logtoxml',('"'+$log.FullName+'"'),('"'+$xmlPath+'"') -WindowStyle Hidden -Wait
        [xml]$xml = Get-Content -LiteralPath $xmlPath -Raw
        if ($xml.SelectNodes("//*[local-name()='logEntry']").Count -ne 0) { throw "Verifier stop recorded in $xmlPath" }
    }
    Write-Output "Heap/handle verification passed. Logs: $testRoot"
} finally {
    Start-Process -FilePath $verifier -ArgumentList '-delete','settings','-for',$testName -WindowStyle Hidden -Wait
    $env:VERIFIER_LOG_PATH = $previousLogPath
    Pop-Location
}
