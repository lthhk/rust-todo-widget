$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$OutDir = Join-Path $Root "dist"
$Exe = Join-Path $OutDir "todo_widget.exe"

# 清理旧构建（可选）
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $OutDir
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# 使用 cargo 构建 release 版本
Push-Location $Root
cargo build --release
Pop-Location

# 复制生成的 exe
$CargoExe = Join-Path $Root "target\release\todo_widget.exe"
if (-not (Test-Path $CargoExe)) {
    Write-Error "编译失败，未找到 $CargoExe"
    exit 1
}
Copy-Item -LiteralPath $CargoExe -Destination $Exe -Force

Write-Host "Built: $Exe"