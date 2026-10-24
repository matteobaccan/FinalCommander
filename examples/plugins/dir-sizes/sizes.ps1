# The same on Windows: every subdirectory measured, biggest first.
$ErrorActionPreference = 'SilentlyContinue'
[Console]::In.ReadToEnd() | Out-Null
$dir = $env:SCMD_DIR
if (-not (Test-Path -LiteralPath $dir)) {
  Write-Output '{"error":"not a directory on this machine"}'
  exit 0
}
$rows = @()
foreach ($folder in Get-ChildItem -LiteralPath $dir -Directory -Force) {
  $files = Get-ChildItem -LiteralPath $folder.FullName -File -Recurse -Force
  $bytes = ($files | Measure-Object -Property Length -Sum).Sum
  if (-not $bytes) { $bytes = 0 }
  $rows += [pscustomobject]@{ Name = $folder.Name; Bytes = [long]$bytes; Count = $files.Count }
}
if ($rows.Count -eq 0) {
  Write-Output '{"status":"no folders here to measure"}'
  exit 0
}
function Format-Size([long]$bytes) {
  $units = 'B', 'K', 'M', 'G', 'T'
  $value = [double]$bytes
  $i = 0
  while ($value -ge 1024 -and $i -lt 4) { $value /= 1024; $i++ }
  if ($value -lt 10 -and $i -gt 0) { '{0:N1} {1}' -f $value, $units[$i] }
  else { '{0:N0} {1}' -f $value, $units[$i] }
}
$lines = $rows | Sort-Object -Property Bytes -Descending | Select-Object -First 40 | ForEach-Object {
  $text = '{0,8}  {1} ({2} files)' -f (Format-Size $_.Bytes), $_.Name, $_.Count
  '"' + ($text -replace '\\', '\\\\' -replace '"', '\"') + '"'
}
Write-Output ('{"message":{"title":"Folder sizes","lines":[' + ($lines -join ',') + ']}}')
