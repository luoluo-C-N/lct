[CmdletBinding()]
param(
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'

function Invoke-Git {
    param([string[]]$Arguments)

    $gitArguments = @('-C', $script:repositoryPath) + $Arguments
    $result = & git @gitArguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed: $($result | Out-String)"
    }
    return ($result | Out-String).Trim()
}

$repositoryPath = (& git -C $PSScriptRoot rev-parse --show-toplevel | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw 'The refresh script must be stored inside a Git repository.'
}
$dirtyEntries = Invoke-Git @('status', '--porcelain')
if (-not [string]::IsNullOrWhiteSpace($dirtyEntries)) {
    [Console]::Error.WriteLine('Cannot create a handoff from a repository with uncommitted changes. Commit or stash them first.')
    exit 1
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryPath 'handoff'
}

$resolvedOutput = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $resolvedOutput -Force | Out-Null

$branch = Invoke-Git @('branch', '--show-current')
$head = Invoke-Git @('rev-parse', 'HEAD')
$remoteUrl = ''
 $remoteResult = & git -C $repositoryPath remote get-url origin 2>$null
if ($LASTEXITCODE -eq 0) {
    $remoteUrl = ($remoteResult | Out-String).Trim()
}
$sanitizedRemoteUrl = $remoteUrl -replace '^(https?://|ssh://)[^/@]+@', '$1'

$bundlePath = Join-Path $resolvedOutput 'magic-image-library.bundle'
$manifestPath = Join-Path $resolvedOutput 'manifest.json'
$bundleTemporaryPath = Join-Path $resolvedOutput '.magic-image-library.bundle.tmp'
$manifestTemporaryPath = Join-Path $resolvedOutput '.manifest.json.tmp'

Remove-Item -LiteralPath $bundleTemporaryPath, $manifestTemporaryPath -Force -ErrorAction SilentlyContinue
try {
    Invoke-Git @('bundle', 'create', $bundleTemporaryPath, '--branches', '--tags') | Out-Null
    Invoke-Git @('bundle', 'verify', $bundleTemporaryPath) | Out-Null

    [pscustomobject]@{
        branch = $branch
        head = $head
        generatedAt = [DateTime]::UtcNow.ToString('o')
        remoteUrl = $sanitizedRemoteUrl
    } | ConvertTo-Json | Set-Content -LiteralPath $manifestTemporaryPath -Encoding utf8NoBOM

    Move-Item -LiteralPath $bundleTemporaryPath -Destination $bundlePath -Force
    Move-Item -LiteralPath $manifestTemporaryPath -Destination $manifestPath -Force
} finally {
    Remove-Item -LiteralPath $bundleTemporaryPath, $manifestTemporaryPath -Force -ErrorAction SilentlyContinue
}

Write-Output "Handoff refreshed: $resolvedOutput"
