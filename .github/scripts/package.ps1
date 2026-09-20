$ErrorActionPreference = 'Stop'
$version = (Get-Content -LiteralPath '.clean-dev-cycle-release.json' -Raw | ConvertFrom-Json).version
if (-not $version) { throw 'O manifesto de release não informa a versão.' }
if ($IsWindows) {
    $triple = 'x86_64-pc-windows-gnu'
    $binary = 'clean-dev-cycle.exe'
} elseif ($IsLinux) {
    $triple = 'x86_64-unknown-linux-gnu'
    $binary = 'clean-dev-cycle'
} else { throw 'Este empacotador suporta Linux e Windows x86_64.' }
$source = Join-Path 'target/release' $binary
if (-not (Test-Path -LiteralPath $source)) { throw "Binário ausente: $source. Execute cargo build --release --locked." }
New-Item -ItemType Directory -Force 'dist' | Out-Null
$name = "clean-dev-cycle-v$version-$triple"
if ($IsWindows) {
    $archive = Join-Path 'dist' "$name.zip"
    Compress-Archive -LiteralPath $source -DestinationPath $archive -Force
} else {
    $archive = Join-Path 'dist' "$name.tar.gz"
    tar -czf $archive -C 'target/release' $binary
    if ($LASTEXITCODE -ne 0) { throw 'Falha ao empacotar o binário.' }
}
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
$file = Split-Path -Leaf $archive
[IO.File]::WriteAllText("$((Resolve-Path -LiteralPath $archive).Path).sha256", "$hash  $file`n")
Get-ChildItem -LiteralPath 'dist' -File | ForEach-Object { "$($_.Name) ($($_.Length) bytes)" }
