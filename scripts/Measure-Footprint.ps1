param(
    [string]$Executable = "$PSScriptRoot\..\target\release\orbit.exe",
    [int]$WarmupSeconds = 8,
    [int]$SampleSeconds = 20,
    [string]$Output = "$PSScriptRoot\..\artifacts\footprint.json"
)
$ErrorActionPreference = 'Stop'
if (-not ('OrbitMemory' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class OrbitMemory {
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("psapi.dll", SetLastError=true)] static extern bool QueryWorkingSet(IntPtr process, IntPtr buffer, int size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetProcessTimes(IntPtr process, out long created, out long exited, out long kernel, out long user);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool QueryProcessCycleTime(IntPtr process, out ulong cycles);
    public static double CpuMilliseconds(int pid) {
        IntPtr process=OpenProcess(0x1000,false,pid);
        if(process==IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        try {
            long created,exited,kernel,user;
            if(!GetProcessTimes(process,out created,out exited,out kernel,out user)) throw new System.ComponentModel.Win32Exception();
            return (kernel+user)/10000.0;
        } finally {CloseHandle(process);}
    }
    public static ulong CpuCycles(int pid) {
        IntPtr process=OpenProcess(0x1000,false,pid);
        if(process==IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        try {
            ulong cycles;
            if(!QueryProcessCycleTime(process,out cycles)) throw new System.ComponentModel.Win32Exception();
            return cycles;
        } finally {CloseHandle(process);}
    }
    public static long PrivateWorkingSet(int pid) {
        IntPtr process=OpenProcess(0x0410,false,pid);
        if(process==IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        int bytes=1024*1024;
        try {
            for(int attempt=0;attempt<4;attempt++,bytes*=2) {
                IntPtr buffer=Marshal.AllocHGlobal(bytes);
                try {
                    if(!QueryWorkingSet(process,buffer,bytes)) {
                        int error=Marshal.GetLastWin32Error();
                        if(error==24) continue;
                        throw new System.ComponentModel.Win32Exception(error);
                    }
                    long count=Marshal.ReadIntPtr(buffer).ToInt64(), privatePages=0;
                    for(int i=0;i<count;i++) {
                        long block=Marshal.ReadIntPtr(buffer,IntPtr.Size*(i+1)).ToInt64();
                        if((block&0x100)==0) privatePages++;
                    }
                    return privatePages*Environment.SystemPageSize;
                } finally {Marshal.FreeHGlobal(buffer);}
            }
            throw new Exception("Working set is too large for this measurement buffer.");
        } finally {CloseHandle(process);}
    }
}
'@
}
$resolvedExe = (Resolve-Path -LiteralPath $Executable).Path
$artifactRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\artifacts'))
New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
$scenarios = @(
    @{ Name = 'tray-idle'; Options = @('--tray') },
    @{ Name = 'tray-running'; Options = @('--tray', '--running') },
    @{ Name = 'main-paused'; Options = @() },
    @{ Name = 'main-running'; Options = @('--running') },
    @{ Name = 'main-minimized-running'; Options = @('--running', '--minimized') },
    @{ Name = 'all-widgets-running'; Options = @('--all-widgets', '--running') }
)
$results = foreach ($scenario in $scenarios) {
    $scenarioData = Join-Path $artifactRoot ('measure-' + $scenario.Name + '-' + [Guid]::NewGuid().ToString('N'))
    $arguments = @('--isolated', '--data-dir', ('"' + $scenarioData + '"'), '--quit-after', ($WarmupSeconds + $SampleSeconds + 5)) + $scenario.Options
    $started = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $resolvedExe -ArgumentList $arguments -WindowStyle Hidden -PassThru
    try {
        $inputIdle = $process.WaitForInputIdle(10000)
        $inputIdleMs = if ($inputIdle) { $started.Elapsed.TotalMilliseconds } else { $null }
        Start-Sleep -Seconds $WarmupSeconds
        $process.Refresh()
        if ($process.HasExited) { throw "App exited during $($scenario.Name); code $($process.ExitCode)." }
        $cpuStart = [OrbitMemory]::CpuMilliseconds($process.Id)
        $cycleStart = [OrbitMemory]::CpuCycles($process.Id)
        $privateSamples = [Collections.Generic.List[long]]::new()
        $workingSamples = [Collections.Generic.List[long]]::new()
        $privateWorkingSamples = [Collections.Generic.List[long]]::new()
        $sampleClock = [Diagnostics.Stopwatch]::StartNew()
        for ($index = 0; $index -lt $SampleSeconds; $index++) {
            Start-Sleep -Seconds 1
            $process.Refresh()
            if ($process.HasExited) { throw "App exited while measuring $($scenario.Name)." }
            $privateSamples.Add($process.PrivateMemorySize64)
            $workingSamples.Add($process.WorkingSet64)
            $privateWorkingSamples.Add([OrbitMemory]::PrivateWorkingSet($process.Id))
        }
        $elapsed = $sampleClock.Elapsed.TotalMilliseconds
        $process.Refresh()
        $cpuMs = [OrbitMemory]::CpuMilliseconds($process.Id) - $cpuStart
        $cycleDelta = [OrbitMemory]::CpuCycles($process.Id) - $cycleStart
        $cpu = 100.0 * $cpuMs / $elapsed
        [pscustomobject]@{
            scenario = $scenario.Name
            privateBytesMean = [long](($privateSamples | Measure-Object -Average).Average)
            privateBytesPeak = [long](($privateSamples | Measure-Object -Maximum).Maximum)
            workingSetBytesMean = [long](($workingSamples | Measure-Object -Average).Average)
            privateWorkingSetBytesMean = [long](($privateWorkingSamples | Measure-Object -Average).Average)
            cpuPercentOfOneCore = [Math]::Round($cpu, 4)
            cpuMilliseconds = $cpuMs
            cpuCycles = $cycleDelta
            sampleSeconds = $SampleSeconds
            warmupSeconds = $WarmupSeconds
            startupToNativeInputIdleMs = $inputIdleMs
        }
    }
    finally {
        if (-not $process.HasExited) { $process.WaitForExit(12000) | Out-Null }
        if (-not $process.HasExited) { Stop-Process -Id $process.Id }
    }
}
$report = [pscustomobject]@{
    measuredAt = [DateTimeOffset]::Now.ToString('o')
    os = [Environment]::OSVersion.VersionString
    logicalProcessors = [Environment]::ProcessorCount
    executable = $resolvedExe
    executableBytes = (Get-Item -LiteralPath $resolvedExe).Length
    executableSha256 = (Get-FileHash -LiteralPath $resolvedExe -Algorithm SHA256).Hash
    note = 'CPU uses GetProcessTimes directly and is relative to one logical core; zero means below the accounting resolution during this sample. QueryProcessCycleTime deltas are also recorded. Private bytes are committed private memory; working set includes shared resident pages. Private working set uses QueryWorkingSet non-shared pages. No working-set trimming is performed. Each scenario uses an empty isolated database. Input-idle is not first paint.'
    scenarios = @($results)
}
$outputPath = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Path (Split-Path -Parent $outputPath) -Force | Out-Null
$report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $outputPath -Encoding utf8
$report.scenarios | Format-Table scenario,privateBytesMean,workingSetBytesMean,cpuPercentOfOneCore
Write-Output "Report: $outputPath"
