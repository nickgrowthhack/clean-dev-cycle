param([Parameter(Mandatory)][string]$Binary)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
if ($env:GITHUB_REF -ne 'refs/heads/main') { throw 'Execute o CI na main.' }
$head = $env:GITHUB_SHA
if ($head -notmatch '^[0-9a-f]{40}$') { throw 'SHA do evento inválido.' }
switch ($env:GITHUB_EVENT_NAME) {
    'push' {
        $event = Get-Content -LiteralPath $env:GITHUB_EVENT_PATH -Raw | ConvertFrom-Json
        if ($event.after -cne $head) { throw 'O SHA não corresponde ao push.' }
        $base = $event.before
    }
    'workflow_dispatch' {
        $base = git rev-parse "${head}^"
        if ($LASTEXITCODE -ne 0) { throw 'Não foi possível resolver o pai do commit.' }
    }
    default { throw 'Evento de CI não suportado.' }
}
if ($base -notmatch '^[0-9a-f]{40}$' -or $base -eq ('0' * 40)) {
    throw 'O CI exige um commit base existente na main.'
}
git merge-base --is-ancestor $base $head
if ($LASTEXITCODE -ne 0) { throw 'O intervalo não é um avanço da main.' }
& $Binary check-commit --from $base --to $head
if ($LASTEXITCODE -ne 0) { throw 'Há mensagens inválidas no intervalo enviado.' }
