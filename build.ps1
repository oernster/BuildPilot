# The build. Reads VERSION, stamps it into Cargo.toml, runs the gate, builds the application,
# then builds the setup program carrying it.
#
#   ./build.ps1                   the application and the setup program
#   ./build.ps1 -SkipInstaller    the application only
#
# The gate cannot be skipped: a gate that can be skipped is skipped on the day it would have
# caught something. Each step fails the script outright.
param(
    [switch]$SkipInstaller
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

$appName = 'buildpilot'
$setupName = 'buildpilotsetup'
$distFolder = Join-Path $root 'dist'

function Invoke-Step([string]$name, [scriptblock]$step) {
    Write-Host "$name..."
    & $step
    if ($LASTEXITCODE -ne 0) {
        Write-Host ''
        Write-Host "$name FAILED." -ForegroundColor Red
        exit 1
    }
}

# 1. The version, from its one home.
$version = (Get-Content (Join-Path $root 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "VERSION holds '$version', which is not a version of the form 1.2.3."
}
Write-Host "Building BuildPilot $version"

# 2. Cargo.toml carries the same version (CON-005, OQ-12): the package's own line only.
$manifestPath = Join-Path $root 'Cargo.toml'
$manifest = Get-Content $manifestPath -Raw
$stamped = [regex]::Replace($manifest, '(?m)^version = "[^"]*"', "version = `"$version`"", 1)
if ($stamped -ne $manifest) {
    Set-Content -Path $manifestPath -Value $stamped -NoNewline
    Write-Host "Stamped Cargo.toml with $version"
}

# 3. The gate.
Invoke-Step 'Running the gate' { & (Join-Path $root 'test.ps1') }

# 4. The application.
Invoke-Step 'Building the application' { cargo build --release --bin $appName }
$app = Join-Path $root "target\release\$appName.exe"

if ($SkipInstaller) {
    Write-Host ''
    Write-Host "Built $app" -ForegroundColor Green
    exit 0
}

# 5. The setup program, carrying the application built in step 4.
$env:BUILDPILOT_PAYLOAD = $app
try {
    Invoke-Step 'Building the setup program' { cargo build --release --bin $setupName }
}
finally {
    Remove-Item Env:\BUILDPILOT_PAYLOAD
}

# 6. The distribution.
New-Item -ItemType Directory -Force $distFolder | Out-Null
$setup = Join-Path $distFolder 'BuildPilotSetup.exe'
Copy-Item (Join-Path $root "target\release\$setupName.exe") $setup -Force

Write-Host ''
Write-Host "Built $app" -ForegroundColor Green
Write-Host "Built $setup" -ForegroundColor Green
