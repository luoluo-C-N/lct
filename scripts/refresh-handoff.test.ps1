$ErrorActionPreference = 'Stop'

$sourceScript = Join-Path $PSScriptRoot 'refresh-handoff.ps1'
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("magic-image-library-handoff-test-{0}" -f [guid]::NewGuid())
$repoPath = Join-Path $tempRoot 'repository'
$outputPath = Join-Path $tempRoot 'handoff'

try {
    New-Item -ItemType Directory -Path $repoPath -Force | Out-Null
    git -C $repoPath init --quiet
    git -C $repoPath config user.name 'Handoff Test'
    git -C $repoPath config user.email 'handoff-test@example.invalid'
    git -C $repoPath config core.autocrlf false
    Set-Content -LiteralPath (Join-Path $repoPath 'README.md') -Value '# fixture' -NoNewline
    Copy-Item -LiteralPath $sourceScript -Destination (Join-Path $repoPath 'refresh-handoff.ps1')
    git -C $repoPath add README.md refresh-handoff.ps1
    git -C $repoPath commit --quiet -m 'fixture'

    & (Join-Path $repoPath 'refresh-handoff.ps1') -OutputDirectory $outputPath
    if ($LASTEXITCODE -ne 0) { throw 'The handoff refresh script failed for a clean repository.' }

    $bundlePath = Join-Path $outputPath 'magic-image-library.bundle'
    $manifestPath = Join-Path $outputPath 'manifest.json'
    if (-not (Test-Path -LiteralPath $bundlePath)) { throw 'The handoff bundle was not created.' }
    if (-not (Test-Path -LiteralPath $manifestPath)) { throw 'The handoff manifest was not created.' }
    git -C $repoPath bundle verify $bundlePath 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The generated bundle did not verify.' }

    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.branch -ne 'master') { throw "Expected master branch, got $($manifest.branch)." }
    if ([string]::IsNullOrWhiteSpace($manifest.head) -or [string]::IsNullOrWhiteSpace($manifest.generatedAt)) { throw 'Manifest is missing required snapshot fields.' }
    if ((Get-Content -LiteralPath $manifestPath -Raw) -match '(?i)token|password|proxy') { throw 'Manifest contains a sensitive configuration field.' }

    $beforeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $bundlePath).Hash
    Set-Content -LiteralPath (Join-Path $repoPath 'unsaved.txt') -Value 'dirty' -NoNewline
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    $dirtyOutput = & pwsh -NoProfile -File (Join-Path $repoPath 'refresh-handoff.ps1') -OutputDirectory $outputPath 2>&1
    $dirtyExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    if ($dirtyExitCode -eq 0) { throw 'The script accepted a dirty repository.' }
    if ($dirtyOutput -notmatch 'uncommitted') { throw 'The dirty-repository failure was not actionable.' }
    $afterHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $bundlePath).Hash
    if ($beforeHash -ne $afterHash) { throw 'A dirty repository overwrote an existing bundle.' }

    Write-Output 'PASS: handoff bundle creation, manifest safety, and dirty-tree protection verified.'
}
finally {
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force }
}
