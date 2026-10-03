# The gate. Checks formatting, runs clippy with warnings as errors, then runs every test under
# coverage and fails below 100% line or region coverage of the correctness core: src/domain,
# src/application and src/setup. Infrastructure, UI, src/bin and src/main.rs are measured but sit
# outside the floor: they need a real desktop, processes and files, so a number over them would
# mean little.
#
#   ./test.ps1          run the whole gate
#   ./test.ps1 -Html    also open the HTML coverage report
#
# The cheap checks run first: there is no sense waiting out a coverage build to be told about
# formatting. Each step fails the script outright, so a green run means every step passed.
param(
    [switch]$Html
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

# Line and region floor over the correctness core (NFR-MAINT-001).
$coverageFloor = 100
# Files outside the floor: infrastructure, UI, src/bin, src/main.rs and the tests themselves.
$outsideFloor = '(src[\\/](infrastructure|ui|bin)[\\/]|src[\\/]main\.rs|tests[\\/])'

function Invoke-Step([string]$name, [scriptblock]$step) {
    Write-Host "$name..."
    & $step
    if ($LASTEXITCODE -ne 0) {
        Write-Host ''
        Write-Host "$name FAILED." -ForegroundColor Red
        exit 1
    }
}

Invoke-Step 'Checking formatting' { cargo fmt --check }
Invoke-Step 'Running clippy' { cargo clippy --all-targets -- -D warnings }
Invoke-Step 'Running tests with coverage' {
    cargo llvm-cov --ignore-filename-regex $outsideFloor --fail-under-lines $coverageFloor `
        --fail-under-regions $coverageFloor
}

if ($Html) {
    cargo llvm-cov report --ignore-filename-regex $outsideFloor --html --open
}

Write-Host ''
Write-Host 'Gate passed.' -ForegroundColor Green
