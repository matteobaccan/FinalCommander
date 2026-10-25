# Rename what is selected so the names are all one case. The Windows half of case.sh.
$ErrorActionPreference = 'Stop'
# The context is on standard input as JSON; this half reads the environment instead, which
# says the same things.
[Console]::In.ReadToEnd() | Out-Null

$dir = $env:FCMD_DIR
if (-not $dir -or -not (Test-Path -LiteralPath $dir)) {
  Write-Output '{"error":"not a directory on this machine"}'
  exit 0
}
Set-Location -LiteralPath $dir

$names = @()
if ($env:FCMD_SELECTED) { $names = $env:FCMD_SELECTED -split "`n" | Where-Object { $_ } }
if (-not $names -and $env:FCMD_CURSOR) { $names = @($env:FCMD_CURSOR) }
if (-not $names) {
  Write-Output '{"status":"nothing to rename"}'
  exit 0
}

$how = switch -Wildcard ($env:FCMD_COMMAND) {
  '*.lower' { 'lower' }
  '*.upper' { 'upper' }
  '*.title' { 'title' }
  default   { $null }
}
if (-not $how) {
  Write-Output '{"error":"this plugin does not know that command"}'
  exit 0
}

$culture = [Globalization.CultureInfo]::InvariantCulture
foreach ($name in $names) {
  if ($name -eq '..') { continue }
  $new = switch ($how) {
    'lower' { $name.ToLowerInvariant() }
    'upper' { $name.ToUpperInvariant() }
    'title' { $culture.TextInfo.ToTitleCase($name.ToLowerInvariant()) }
  }
  if ($new -eq $name) { continue }
  # Windows filesystems are case-insensitive, so renaming A.txt to a.txt is a rename of a file
  # onto itself: Rename-Item handles that, but a *different* name already there is left alone.
  if ((Test-Path -LiteralPath $new) -and ($new.ToLowerInvariant() -ne $name.ToLowerInvariant())) {
    continue
  }
  try { Rename-Item -LiteralPath $name -NewName $new -ErrorAction Stop } catch { }
}

Write-Output '{"run":"panel.reread"}'
Write-Output ('{"status":"renamed to ' + $how + ' case"}')
