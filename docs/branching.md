# Fluxo com Jujutsu e somente main

Trabalhe em uma mudança pequena por vez. Mantenha dependências em um stack local
e integre cada camada assim que ela estiver verificável. Jujutsu é a interface
de trabalho. O Git permanece como armazenamento e transporte para GitHub.

## Caminho cotidiano

1. Edite a mudança atual (`@`) e confira `jj diff`.
2. Separe resultados independentes com `jj split`.
3. Conclua com `clean-dev-cycle commit` ou `jj commit -m MENSAGEM`.
4. Execute `clean-dev-cycle submit`. O padrão é a mudança concluída em `@-`.
5. O submit verifica o commit e publica diretamente na main. Acompanhe o CI.

Use `submit --revision ID` para enviar uma camada anterior. A ferramenta indica
qual camada vem primeiro quando o stack ainda tem uma dependência não integrada.
Não envie `@`, que ainda está em edição. `main` é a única branch local e remota.
Não crie branches por escopo, PRs ou uma branch intermediária de integração.
Os stacks são mudanças locais do Jujutsu, sem bookmarks adicionais.

## O que chega à main

O envio deve conter exatamente um commit filho da main atual, com alteração de
arquivos, mensagem e identidade válidas e sem conflitos. Antes do push, o submit
executa os comandos de `clean-dev-cycle.toml` do commit em um clone temporário,
com HEAD destacado. Falhas ou alterações de arquivos versionados pelos checks
impedem a publicação. Consulte a configuração e os limites no [README](../README.md).

O SHA aprovado localmente é enviado diretamente à main. O CI verifica Linux e
Windows depois da publicação. Ele não promove commits nem bloqueia previamente
o push. Um novo push não cancela a execução anterior. Os checks locais são uma
regra do submit e não substituem uma proteção no servidor contra envios manuais.

Force-push, exclusão da main e histórico não linear permanecem bloqueados,
inclusive para administradores. Changelog, release e deploy não bloqueiam o envio.

## Quando algo falha

- Check local ou mensagem inválida: corrija a mudança com `jj edit ID`, confira o diff,
  conclua novamente e reenvie. Descendentes são reaplicados pelo Jujutsu.
- Base avançou: atualize o remoto e faça rebase da linha local com
  `jj git fetch --remote origin` e `jj rebase -b @ -o main@origin`. Resolva os
  conflitos, revise e reenvie. O novo SHA precisa de novos checks.
- Mudança reescrita durante os checks: revise a nova versão e execute submit
  novamente. O resultado anterior não é reaproveitado.
- Mudança já integrada: `submit` informa isso e não repete checks nem push.
- CI falhou ou houve regressão publicada: prepare uma correção ou reversão como
  nova mudança e envie pelo mesmo fluxo. Não reescreva a main.

Confira `jj op log` para investigar operações locais e `jj undo` para desfazer
a última operação apropriada. Uma operação local não desfaz uma integração
já publicada na main.

## Migração do fluxo anterior

O bookmark intermediário `nick/submit` e a promoção pelo CI foram retirados.
A proteção deixou de exigir o check prévio `Qualidade` para permitir o push
direto. As outras proteções da main foram mantidas. A branch intermediária só
pode ser excluída depois de confirmar que não contém commits exclusivos.

`commit` passou a operar sobre `@`, sem stage ou hooks Git. `check-ci` foi removido.
`changelog --base ... --pr ... --check` foi substituído pela síntese opcional
`changelog --from ... --to ...`. O histórico do changelog foi mantido.

As branches antigas e os quatro worktrees auxiliares foram arquivados e retirados
do workspace ativo. O backup local verificado está em
`.tools/archive/2026-09-19-jj/repository.bundle`, acompanhado do inventário de SHAs.
Ele foi restaurado em um repositório separado antes da limpeza. Esse arquivo é
local, ignorado pelo Git, e não acompanha novos clones.

Arquivamento não significa integração. Sete branches tiveram suas entregas
integradas pelos PRs #2 a #8: `branching-strategy`, `commit-cli`, `changelog`,
`commit-validation-simple`, `changelog-manual`, `ci-simple` e `main-protection`.
Os commits resultantes continuam no histórico da main, mesmo onde esta migração
substituiu o comportamento anterior.

As branches `commit-validation` e `ci-delivery` ficaram somente no arquivo.
A política configurável de mensagens e as regras daquele CI já haviam sido
descartadas na simplificação anterior. Seus commits originais estão preservados
no bundle. Todas as nove tinham o prefixo `nick/initial-cycle/`. O `README.md`
do arquivo registra os SHAs, as integrações e o comando de restauração.

## Tempos históricos da migração anterior

Na última entrega anterior, o [CI do PR #8](https://github.com/nickgrowthhack/clean-dev-cycle/actions/runs/35406796743)
levou 3min28s e o [CI após o merge](https://github.com/nickgrowthhack/clean-dev-cycle/actions/runs/35407052355)
levou 2min08s: 5min36s somados, sem contar a espera entre execuções.
O [envio da CLI com Jujutsu](https://github.com/nickgrowthhack/clean-dev-cycle/actions/runs/35410391776)
levou 2min16s, incluindo a promoção automática, sem uma segunda suíte na main.

Essas medidas descrevem o antigo fluxo de promoção. São amostras de mudanças
diferentes, medidas entre criação e conclusão dos runs,
com caches e cargas de runners potencialmente distintos. Registram o resultado
observado da migração, sem estabelecer um ganho de desempenho garantido.
