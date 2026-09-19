param(
    [Parameter(Mandatory)][string]$Base,
    [Parameter(Mandatory)][string]$Candidate
)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/check-candidate.ps1" -Base $Base -Candidate $Candidate
git fetch origin main nick/submit
if ($LASTEXITCODE -ne 0) { throw 'Falha ao atualizar as referências remotas.' }
$current = git rev-parse origin/main
if ($LASTEXITCODE -ne 0) { throw 'Não foi possível ler a main.' }
if ($current -ceq $Candidate) {
    Write-Output 'Mudança já integrada.'
    exit 0
}
$submitted = git rev-parse origin/nick/submit
if ($LASTEXITCODE -ne 0 -or $submitted -cne $Candidate) {
    throw 'Este envio foi substituído. Não integrar a execução antiga.'
}
if ($current -cne $Base) { throw 'A main avançou. Faça rebase e reenvie para novo CI.' }
# A normal push is atomic at the server and refuses a concurrent divergent update.
git push origin "${Candidate}:refs/heads/main"
if ($LASTEXITCODE -ne 0) { throw 'A promoção foi recusada. A main não foi reescrita.' }
