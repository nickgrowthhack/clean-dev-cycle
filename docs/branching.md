# Fluxo com Jujutsu

Trabalhe em uma mudança pequena por vez. Mantenha dependências em um stack local
e integre cada camada assim que ela estiver verificável. Jujutsu é a interface
de trabalho. O Git permanece como armazenamento e transporte para GitHub.

## Caminho cotidiano

1. Edite a mudança atual (`@`) e confira `jj diff`.
2. Separe resultados independentes com `jj split`.
3. Conclua com `clean-dev-cycle commit` ou `jj commit -m MENSAGEM`.
4. Execute `clean-dev-cycle submit`. O padrão é a mudança concluída em `@-`.
5. Continue a próxima camada enquanto o CI verifica o envio.

Use `submit --revision ID` para enviar uma camada anterior. A ferramenta indica
qual camada vem primeiro quando o stack ainda tem uma dependência não integrada.
Não envie `@`, que ainda está em edição. Não há branches por escopo, worktrees por
entrega, PR obrigatório ou fila própria. `main` e o bookmark `nick/submit` bastam.

## O que chega à main

O envio deve conter exatamente um commit filho da main atual, com alteração de
arquivos, mensagem válida e sem conflitos. O CI verifica o SHA exato enviado.
Somente após Linux, Windows e `Qualidade` passarem ele promove esse SHA por
fast-forward. A promoção não cria merge nem reescreve commits.

A proteção exige `Qualidade` do GitHub Actions, inclusive para administrador.
Force-push, exclusão da main e histórico não linear permanecem bloqueados.
Changelog, release e deploy não bloqueiam a integração. A configuração remota
da proteção deve acompanhar o nome do check no workflow.

## Quando algo falha

- Teste ou mensagem inválida: corrija a mudança com `jj edit ID`, confira o diff,
  conclua novamente e reenvie. Descendentes são reaplicados pelo Jujutsu.
- Base avançou: atualize o remoto e faça rebase da linha local com
  `jj git fetch --remote origin` e `jj rebase -b @ -o main@origin`. Resolva os
  conflitos, revise e reenvie. O novo SHA precisa de novos checks.
- Envio substituído: acompanhe a execução mais recente. Uma execução antiga
  não promove se detectar outro candidato no bookmark remoto.
- Mudança já integrada: `submit` informa isso e não faz outro push.
- Regressão integrada: prepare uma correção ou reversão como nova mudança e
  passe pelo mesmo CI. Não reescreva a main.

Confira `jj op log` para investigar operações locais e `jj undo` para desfazer
a última operação apropriada. Uma operação local não desfaz uma integração
já publicada na main.

PRs e stacked PRs podem ser adotados para revisão colaborativa quando houver
essa necessidade. Não fazem parte do caminho obrigatório deste fluxo individual.

## Migração do fluxo anterior

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

## Tempos observados

Na última entrega anterior, o [CI do PR #8](https://github.com/nickgrowthhack/clean-dev-cycle/actions/runs/35406796743)
levou 3min28s e o [CI após o merge](https://github.com/nickgrowthhack/clean-dev-cycle/actions/runs/35407052355)
levou 2min08s: 5min36s somados, sem contar a espera entre execuções.
O [envio da CLI com Jujutsu](https://github.com/nickgrowthhack/clean-dev-cycle/actions/runs/35410391776)
levou 2min16s, incluindo a promoção automática, sem uma segunda suíte na main.

São amostras de mudanças diferentes, medidas entre criação e conclusão dos runs,
com caches e cargas de runners potencialmente distintos. Registram o resultado
observado da migração, sem estabelecer um ganho de desempenho garantido.
