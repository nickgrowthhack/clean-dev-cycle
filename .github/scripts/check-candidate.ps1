param(
    [Parameter(Mandatory)][string]$Base,
    [Parameter(Mandatory)][string]$Candidate
)
$ErrorActionPreference = 'Stop'
foreach ($revision in @($Base, $Candidate)) {
    if ($revision -notmatch '^[0-9a-f]{40}$') { throw 'Informe SHAs completos.' }
}
$parents = git show -s --format=%P $Candidate
if ($LASTEXITCODE -ne 0 -or $parents -cne $Base) {
    throw 'Envie exatamente uma mudança filha da main atual. Faça rebase e reenvie.'
}
$object = git cat-file commit $Candidate
if ($LASTEXITCODE -ne 0) { throw 'Não foi possível ler o commit.' }
$headers = ($object -join "`n").Split("`n`n", 2)[0]
if ($headers -match '(?m)^jj:trees ') { throw 'A mudança contém conflitos do Jujutsu.' }
git diff --quiet $Base $Candidate --
if ($LASTEXITCODE -ne 1) { throw 'A mudança deve conter alterações de arquivos.' }
# git diff uses 1 for a real diff; pwsh runners must receive success here.
$global:LASTEXITCODE = 0
