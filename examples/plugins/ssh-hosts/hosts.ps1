# The same list on Windows, where OpenSSH keeps the file in the same place.
$ErrorActionPreference = 'SilentlyContinue'
[Console]::In.ReadToEnd() | Out-Null
$config = Join-Path $env:USERPROFILE '.ssh\config'
if (-not (Test-Path -LiteralPath $config)) {
  Write-Output '{"status":"no ~/.ssh/config here"}'
  exit 0
}
$hosts = @()
$current = $null
foreach ($line in Get-Content -LiteralPath $config) {
  $text = $line.Trim()
  if ($text -eq '' -or $text.StartsWith('#')) { continue }
  $parts = $text -split '\s+', 2
  $key = $parts[0].ToLower()
  $value = if ($parts.Count -gt 1) { $parts[1].Trim() } else { '' }
  switch ($key) {
    'host' {
      if ($current -and $current.Host -notmatch '[\*\?]') { $hosts += $current }
      $current = [pscustomobject]@{ Host = $value; Name = ''; User = '-'; Port = '22' }
    }
    'hostname' { if ($current) { $current.Name = $value } }
    'user' { if ($current) { $current.User = $value } }
    'port' { if ($current) { $current.Port = $value } }
  }
}
if ($current -and $current.Host -notmatch '[\*\?]') { $hosts += $current }
if ($hosts.Count -eq 0) {
  Write-Output '{"status":"no named hosts in ~/.ssh/config"}'
  exit 0
}
$lines = $hosts | ForEach-Object {
  $name = if ($_.Name -eq '') { $_.Host } else { $_.Name }
  $text = '{0,-20} {1}@{2}:{3}' -f $_.Host, $_.User, $name, $_.Port
  '"' + ($text -replace '\\', '\\\\' -replace '"', '\"') + '"'
}
Write-Output ('{"message":{"title":"SSH hosts (' + $hosts.Count + ')","lines":[' + ($lines -join ',') + ']}}')
