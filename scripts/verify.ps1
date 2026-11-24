param([switch]$SkipWindow, [string]$EvidencePrefix = 'root2', [string]$BuildTargetDir = 'target')
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $workspace
$logs = Join-Path $workspace 'evidence\verification'
if ($EvidencePrefix -notmatch '^[a-z][a-z0-9-]{0,31}$') { throw 'invalid evidence prefix' }
$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $workspace 'target'))
$buildTarget = [System.IO.Path]::GetFullPath((Join-Path $workspace $BuildTargetDir))
if ($buildTarget -ne $targetRoot -and -not $buildTarget.StartsWith($targetRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'build target must remain in the workspace target directory' }
$appExecutable = Join-Path $buildTarget 'release\opensequoia.exe'
New-Item -ItemType Directory -Force -Path $logs | Out-Null
function Invoke-Checked {
    param([string]$Name, [string]$File, [string[]]$Arguments)
    & $File @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $logs ($Name + '.log'))
    if ($LASTEXITCODE -ne 0) { throw "$Name failed (exit $LASTEXITCODE)" }
}
Invoke-Checked "$EvidencePrefix-format" 'cargo' @('fmt','--all','--','--check')
Invoke-Checked "$EvidencePrefix-clippy" 'cargo' @('clippy','--workspace','--all-targets','--locked','--','-D','warnings')
Invoke-Checked "$EvidencePrefix-tests" 'cargo' @('test','--workspace','--locked')
Invoke-Checked "$EvidencePrefix-release-build" 'cargo' @('build','--release','--locked','--target-dir',$buildTarget)
Invoke-Checked "$EvidencePrefix-inspect" $appExecutable @('--inspect')
Invoke-Checked "$EvidencePrefix-process-smoke" $appExecutable @('--process-smoke')
if (-not $SkipWindow) {
    Invoke-Checked "$EvidencePrefix-window-smoke" $appExecutable @('--smoke-test')
    Invoke-Checked "$EvidencePrefix-app-smoke" $appExecutable @('--app-smoke')
}
$inputs = Get-ChildItem -LiteralPath (Join-Path $workspace 'crates') -Recurse -File |
    Where-Object { $_.Extension -in @('.rs','.toml','.rc','.manifest') } |
    ForEach-Object { [ordered]@{ path=$_.FullName.Substring($workspace.Length + 1); sha256=(Get-FileHash -LiteralPath $_.FullName).Hash } }
$inputs += @('Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo\config.toml','fixtures\welcome.html','fixtures\guide.html') |
    ForEach-Object { [ordered]@{ path=$_; sha256=(Get-FileHash -LiteralPath (Join-Path $workspace $_)).Hash } }
[ordered]@{ schema=1; verified_at=(Get-Date).ToUniversalTime().ToString('o'); input_hashes=$inputs; build_target=$buildTarget; executable=$appExecutable; window_smoke_run=(-not $SkipWindow); limitations=@('No WPT/Test262/Wasm run','No sandbox, network, full UI/accessibility or browser parity verdict') } |
    ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $logs "$EvidencePrefix-verification.json")
'DEVELOPMENT_GATES_OK'
