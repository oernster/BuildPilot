# Writes the version held in VERSION into every place that cannot read it.
#
# Two kinds of place (CON-005):
#
#   Cargo.toml       the package's own `version = "..."` line
#   docs/            the GitHub Pages site, where each mention of the version is a token:
#
#                        <!--VERSION-->MAJOR.MINOR.PATCH<!--/VERSION-->
#
# Root markdown is never stamped: no document outside the site names a version. The script is
# idempotent: a file already carrying the current version is not written at all. build.ps1 runs it
# first, so a build cannot ship a site or a manifest naming another version.
#
# Usage, from the repository root:
#
#     ./stamp_version.ps1

$ErrorActionPreference = 'Stop'

$versionFile = Join-Path $PSScriptRoot 'VERSION'
$manifest    = Join-Path $PSScriptRoot 'Cargo.toml'
$docsDir     = Join-Path $PSScriptRoot 'docs'

# A release version is MAJOR.MINOR.PATCH. Anything else is refused rather than written into the
# site, where it would read as a real release.
$versionShape = '^\d+\.\d+\.\d+$'
$siteToken    = '(?<open><!--VERSION-->).*?(?<close><!--/VERSION-->)'
$packageLine  = '(?m)^version = "[^"]*"'
$bom          = [System.Text.UTF8Encoding]::new($true).GetPreamble()

$version = (Get-Content -LiteralPath $versionFile -Raw).Trim()
if ($version -notmatch $versionShape) {
    Write-Host "VERSION holds '$version', not MAJOR.MINOR.PATCH; nothing stamped." -ForegroundColor Red
    exit 1
}

# Rewrites `$file` through `$stamp`, keeping its byte order mark exactly as found, so a stamp
# changes the version and nothing else. Answers whether the file was written.
function Update-Stamp([string]$file, [scriptblock]$stamp) {
    $bytes = [System.IO.File]::ReadAllBytes($file)
    $hasBom = $bytes.Length -ge $bom.Length -and
        -not (Compare-Object $bytes[0..($bom.Length - 1)] $bom -SyncWindow 0)
    $encoding = [System.Text.UTF8Encoding]::new($hasBom)
    $text = [System.IO.File]::ReadAllText($file, $encoding)
    $stamped = & $stamp $text
    if ($stamped -eq $text) {
        return $false
    }
    [System.IO.File]::WriteAllText($file, $stamped, $encoding)
    Write-Host "Stamped $version into $(Resolve-Path -Relative $file)"
    return $true
}

$stampedCount = 0

# The package line only: the first `version = ` at the start of a line, never a dependency's.
if (Update-Stamp $manifest { param($text) [regex]::new($packageLine).Replace($text, "version = `"$version`"", 1) }) {
    $stampedCount++
}

if (Test-Path -LiteralPath $docsDir) {
    foreach ($file in Get-ChildItem -LiteralPath $docsDir -Recurse -File -Include *.html, *.md) {
        if (Update-Stamp $file.FullName { param($text) [regex]::Replace($text, $siteToken, "`${open}$version`${close}") }) {
            $stampedCount++
        }
    }
}

if ($stampedCount -eq 0) {
    Write-Host "Version stamps already read $version." -ForegroundColor Gray
}
# An explicit exit code: build.ps1 reads $LASTEXITCODE straight after this runs.
exit 0
