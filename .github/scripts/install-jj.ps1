$ErrorActionPreference = 'Stop'
$version = '0.45.1'
if ($IsWindows) {
    $asset = "jj-v$version-x86_64-pc-windows-msvc.zip"
    $expected = '5dbf2619272c897394d34190a94b21a6ab8d1e8e59fcf7dfc173d8fa8aa90bf0'
} elseif ($IsLinux) {
    $asset = "jj-v$version-x86_64-unknown-linux-musl.tar.gz"
    $expected = 'f35438350b5d61963aac5dd74ede510b31d6b9690769d1a6268cf058cc825f72'
} else { throw 'Este instalador do CI suporta Linux e Windows x86_64.' }
$directory = Join-Path $env:RUNNER_TEMP 'jj'
New-Item -ItemType Directory -Force $directory | Out-Null
$archive = Join-Path $directory $asset
Invoke-WebRequest "https://github.com/jj-vcs/jj/releases/download/v$version/$asset" -OutFile $archive
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expected) {
    throw 'O checksum do Jujutsu não corresponde à versão fixada.'
}
if ($IsWindows) { Expand-Archive -LiteralPath $archive -DestinationPath $directory -Force }
else {
    tar -xzf $archive -C $directory
    if ($LASTEXITCODE -ne 0) { throw 'Falha ao extrair o Jujutsu.' }
}
$binary = Join-Path $directory $(if ($IsWindows) { 'jj.exe' } else { 'jj' })
& $binary --version
if ($LASTEXITCODE -ne 0) { throw 'O Jujutsu não executou.' }
$directory >> $env:GITHUB_PATH
