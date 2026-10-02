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
# The site's local stylesheet and script links also carry their file's content hash, as
# styles.css?v=<hash>. GitHub Pages lets a browser keep a stylesheet for ten minutes, so a fresh
# page could otherwise be drawn with the old one; a changed file is a new address instead.
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

# A local stylesheet or script link: the path, any query it already carries (replaced), then any
# fragment (kept). The hash is the first $assetHashLength hex characters of its SHA-256.
$assetLink       = '(?<attr>\b(?:href|src)=)(?<quote>["''])(?<path>[^"''?#]+\.(?:css|js))(?:\?[^"''#]*)?(?<fragment>#[^"'']*)?\k<quote>'
$assetHashLength = 10

$version = (Get-Content -LiteralPath $versionFile -Raw).Trim()
if ($version -notmatch $versionShape) {
    Write-Host "VERSION holds '$version', not MAJOR.MINOR.PATCH; nothing stamped." -ForegroundColor Red
    exit 1
}

# Rewrites `$file` through `$stamp`, keeping its byte order mark exactly as found, so a stamp
# changes the version and nothing else. `$what` names the stamp in the report. Answers whether the
# file was written.
function Update-Stamp([string]$file, [scriptblock]$stamp, [string]$what = $version) {
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
    Write-Host "Stamped $what into $(Resolve-Path -Relative $file)"
    return $true
}

# The hash a link carries, over the file's bytes with CRLF read as LF, so a Windows checkout and
# the LF blob GitHub serves agree. A link to a missing file stops the stamp rather than hash nothing.
function Get-AssetHash([string]$path) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "A site page links $path, which does not exist; nothing hashed."
    }
    # Latin-1 maps every byte to one character, so the CRLF swap is exact.
    $latin1 = [System.Text.Encoding]::GetEncoding('iso-8859-1')
    $bytes = $latin1.GetBytes($latin1.GetString([System.IO.File]::ReadAllBytes($path)).Replace("`r`n", "`n"))
    $digest = [System.Security.Cryptography.SHA256]::Create().ComputeHash($bytes)
    (-join ($digest | ForEach-Object { $_.ToString('x2') })).Substring(0, $assetHashLength)
}

# Puts each relative link's hash on it, resolved against the page's own folder. Remote,
# protocol-relative and root-absolute links are not this site's files and are left alone.
function Add-AssetHashes([string]$text, [string]$folder) {
    [regex]::Replace($text, $assetLink, {
        param($link)
        $path = $link.Groups['path'].Value
        if ($path.StartsWith('/') -or $path.Contains(':')) {
            return $link.Value
        }
        $quote = $link.Groups['quote'].Value
        '{0}{1}{2}?v={3}{4}{1}' -f $link.Groups['attr'].Value, $quote, $path,
            (Get-AssetHash (Join-Path $folder $path)), $link.Groups['fragment'].Value
    })
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
    foreach ($file in Get-ChildItem -LiteralPath $docsDir -Recurse -File -Include *.html) {
        $folder = $file.DirectoryName
        if (Update-Stamp $file.FullName { param($text) Add-AssetHashes $text $folder } 'asset hashes') {
            $stampedCount++
        }
    }
}

if ($stampedCount -eq 0) {
    Write-Host "Version stamps already read $version; asset hashes are current." -ForegroundColor Gray
}
# An explicit exit code: build.ps1 reads $LASTEXITCODE straight after this runs.
exit 0
