# usage: implib-smoke.ps1 <import-lib> <dll-dir>
# Links implib_smoke.c against the import library, then runs it, so a missing or
# mismatched .dll.lib fails here instead of in a consumer's build.
param(
    [Parameter(Mandatory = $true)][string]$ImportLib,
    [Parameter(Mandatory = $true)][string]$DllDir
)
$ErrorActionPreference = 'Stop'

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = & $vswhere -latest -products * -property installationPath
if (-not $vs) { throw 'Visual Studio installation not found' }
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'

$here = $PSScriptRoot
$header = Resolve-Path (Join-Path $here '..\..\crates\capi')
$work = New-Item -ItemType Directory -Path (Join-Path $env:RUNNER_TEMP 'implib-smoke') -Force
$exe = Join-Path $work 'implib_smoke.exe'
$src = Join-Path $here 'implib_smoke.c'

$env:PATH = "$((Resolve-Path $DllDir).Path);$env:PATH"
$lib = (Resolve-Path $ImportLib).Path
Push-Location $work
try {
    cmd /c "`"$vcvars`" >nul && cl /nologo /I `"$header`" /Fe:`"$exe`" `"$src`" `"$lib`""
    if ($LASTEXITCODE -ne 0) { throw 'link against the import library failed' }
} finally {
    Pop-Location
}
& $exe
if ($LASTEXITCODE -ne 0) { throw 'smoke executable failed to run' }
