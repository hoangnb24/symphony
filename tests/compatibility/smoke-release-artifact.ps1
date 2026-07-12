param(
  [Parameter(Mandatory=$true)][string]$Artifact,
  [Parameter(Mandatory=$true)][string]$Fixture
)
$ErrorActionPreference = 'Stop'
$expected = (Get-Content "$Artifact.sha256" -Raw).Split(' ', [System.StringSplitOptions]::RemoveEmptyEntries)[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 $Artifact).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw 'archive checksum mismatch' }
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("symphony-release-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
$process = $null
try {
  tar -xf $Artifact -C $temp
  $binary = Join-Path $temp 'bin/harness-symphony.exe'
  if (-not (Test-Path $binary)) { throw 'Windows binary missing from archive' }
  & $binary --version
  if ($LASTEXITCODE -ne 0) { throw '--version failed' }
  $version = (& $binary version --json | ConvertFrom-Json)
  if ($version.symphony_version -ne '0.1.1' -or $version.harness_protocol_version -ne 1 -or $version.harness_schema_minimum -ne 1 -or $version.harness_schema_maximum -ne 13 -or $version.current_harness_schema_minimum -ne 12 -or $version.current_harness_schema_maximum -ne 13 -or ($version.supported_harness_cli_versions -join ',') -ne '0.1.14,0.1.15') { throw 'version JSON contract mismatch' }
  $cli = Join-Path $Fixture 'scripts/bin/harness-cli.exe'
  Push-Location $Fixture
  try {
    $contract = (& $cli query contract --json | ConvertFrom-Json)
    $graph = (& $cli query work-graph --json | ConvertFrom-Json)
  } finally {
    Pop-Location
  }
  $required = @('stories.read.v1','stories.write.v1','work-graph.read.v1','story-dependencies.read-write.v1','story-hierarchy.read-write.v1','changesets.apply.v1','changesets.status-sha.v1','isolated-db.v1','isolated-db-snapshot.v1','semantic-operation-log.v1')
  if ($contract.protocol_version -ne 1 -or $contract.operation -ne 'query.contract' -or $contract.result.cli_version -ne '0.1.14' -or $contract.result.protocol_version -ne 1 -or $contract.result.schema_minimum -ne 1 -or $contract.result.schema_maximum -ne 13 -or $contract.result.database_schema_version -ne 13 -or $contract.result.database_state -ne 'current' -or @($required | Where-Object { $_ -notin $contract.result.capabilities }).Count -ne 0) { throw 'Harness contract tuple mismatch' }
  if ($graph.protocol_version -ne 1 -or $graph.operation -ne 'query.work-graph' -or $null -eq $graph.result.revision -or $null -eq $graph.result.stories) { throw 'Harness work-graph JSON contract mismatch' }
  & $binary --repo-root $Fixture doctor
  if ($LASTEXITCODE -ne 0) { throw 'doctor failed' }
  $caller = Join-Path $temp 'caller'
  New-Item -ItemType Directory -Path $caller | Out-Null
  git -C $caller init -q
  $stdout = Join-Path $temp 'web.stdout.log'
  $stderr = Join-Path $temp 'web.stderr.log'
  $process = Start-Process -FilePath $binary -ArgumentList @('--repo-root', $Fixture, 'web', '--host', '127.0.0.1', '--port', '0') -WorkingDirectory $caller -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
  $base = $null
  for ($i = 0; $i -lt 300; $i++) {
    if ($process.HasExited) { throw "Web backend exited early: $(Get-Content $stderr -Raw)" }
    if (Test-Path $stdout) {
      $content = Get-Content $stdout -Raw
      if ($null -ne $content) {
        $match = [regex]::Match($content, 'http://127\.0\.0\.1:\d+')
        if ($match.Success) { $base = $match.Value; break }
      }
    }
    Start-Sleep -Milliseconds 100
  }
  if (-not $base) { throw 'timed out waiting for Web backend' }
  $health = Invoke-RestMethod "$base/health"
  if (-not $health.ok) { throw 'health response was not ok' }
  $board = Invoke-RestMethod "$base/api/board"
  if ($null -eq $board.items) { throw 'board response omitted items' }
  $index = (Invoke-WebRequest "$base/").Content
  if ($index -notmatch '<div id="root"></div>') { throw 'root UI was not served' }
  Write-Output 'Windows release artifact checksum/version/doctor/Web smoke passed'
} finally {
  if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
  Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
}
