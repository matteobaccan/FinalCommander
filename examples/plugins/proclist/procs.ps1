# The Windows half of procs.sh.
[Console]::In.ReadToEnd() | Out-Null

function Send-Message($title, $lines) {
  $json = ($lines | ForEach-Object {
    '"' + ($_ -replace '\\', '\\\\' -replace '"', '\"' -replace "`t", '  ') + '"'
  }) -join ','
  Write-Output ('{"message":{"title":"' + $title + '","lines":[' + $json + ']}}')
}

if ($env:FCMD_COMMAND -like '*.who-has-this') {
  if (-not $env:FCMD_CURSOR) { Write-Output '{"status":"nothing under the cursor"}'; exit 0 }
  # `openfiles` needs the "maintain objects list" flag and administrator rights; Handle from
  # Sysinternals does not ship with Windows. Saying which is missing beats a blank box.
  $out = & openfiles /query /fo table 2>&1 | Select-String -SimpleMatch $env:FCMD_CURSOR
  if (-not $out) {
    Write-Output ('{"status":"nothing found holding ' + $env:FCMD_CURSOR + ' — openfiles needs administrator rights"}')
    exit 0
  }
  Send-Message ("What has " + $env:FCMD_CURSOR + " open") ($out | ForEach-Object { $_.ToString() })
  exit 0
}

$procs = Get-Process | Sort-Object -Property CPU -Descending | Select-Object -First 20 |
  ForEach-Object { '{0,8} {1,8:N1} {2,10:N0}K  {3}' -f $_.Id, $_.CPU, ($_.WorkingSet64 / 1KB), $_.ProcessName }
Send-Message 'What is running (by processor time)' $procs
