$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
$version = (Get-Content -LiteralPath (Join-Path $projectDirectory '.node-version') -Raw).Trim()
if ($version -notmatch '^22\.\d+\.\d+$') { throw 'Expected a pinned Node 22 version in .node-version.' }
if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { throw 'This portable setup supports Windows x64 only. Install the pinned Node version for your architecture.' }
$toolsDirectory = Join-Path $projectDirectory '.tools'
$archiveName = "node-v$version-win-x64.zip"
$runtimeDirectory = Join-Path $toolsDirectory "node-v$version-win-x64"
New-Item -ItemType Directory -Force -Path $toolsDirectory | Out-Null
if (-not (Test-Path -LiteralPath (Join-Path $runtimeDirectory 'node.exe'))) {
    $baseUrl = "https://nodejs.org/dist/v$version"
    $checksums = (Invoke-WebRequest "$baseUrl/SHASUMS256.txt" -UseBasicParsing).Content
    $checksumLine = $checksums -split "`n" | Where-Object { $_.Trim().EndsWith("  $archiveName") }
    if (@($checksumLine).Count -ne 1) { throw 'Could not identify the official Node archive checksum.' }
    $expectedHash = ($checksumLine -split '\s+')[0]
    $archivePath = Join-Path $toolsDirectory $archiveName
    Invoke-WebRequest "$baseUrl/$archiveName" -OutFile $archivePath -UseBasicParsing
    if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash -ne $expectedHash) { throw 'Node archive checksum mismatch.' }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $toolsDirectory -Force
}
$actualVersion = & (Join-Path $runtimeDirectory 'node.exe') --version
if ($actualVersion -ne "v$version") { throw "Unexpected Node version: $actualVersion" }
Write-Output "Project-local Node $actualVersion is ready. System Node and PATH were not changed."
Write-Output "For this terminal: `$env:Path = '$runtimeDirectory;' + `$env:Path"
