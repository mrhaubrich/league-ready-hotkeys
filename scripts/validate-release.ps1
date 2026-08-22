$ErrorActionPreference = 'Stop'

$cargoBin = 'C:\Users\marco\.cargo\bin'
$env:Path = "$cargoBin;$env:Path"

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release

$binary = Join-Path $PSScriptRoot '..\target\release\league-ready-hotkeys.exe'
$binary = [IO.Path]::GetFullPath($binary)
$hash = Get-FileHash -Algorithm SHA256 -LiteralPath $binary
$size = (Get-Item -LiteralPath $binary).Length

Write-Output "release binary: $binary"
Write-Output "size bytes: $size"
Write-Output "sha256: $($hash.Hash)"
