$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$OutDir = Join-Path $Root "dist"
$Exe = Join-Path $OutDir "todo_widget.exe"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
rustc --edition 2024 (Join-Path $Root "src\main.rs") -o $Exe

Write-Host "Built: $Exe"
