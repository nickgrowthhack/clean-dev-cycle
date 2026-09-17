# initial-cycle: primeiro ciclo completo de desenvolvimento

**Responsável pelo escopo:** Nick (`nick`). **Estado:** em andamento.

O objetivo é comprovar o primeiro ciclo de commits, validação, changelog,
versionamento, tag e GitHub Release, dividindo o trabalho em entregas pequenas.
Seguimos o [guia de branching](../branching.md).

## Critério de conclusão

O escopo termina quando suas entregas obrigatórias estiverem integradas e houver
evidência de um ciclo real: commit revisado, PR validado, síntese da entrega no
changelog, versão determinada, tag e GitHub Release correspondentes. A entrega
de releases deve comprovar também uma retomada sem publicação duplicada.

Código existente numa branch, um workflow escrito ou uma configuração proposta
não comprovam esse resultado. Versionamento e releases ainda precisam de seu
próprio plano e PR, dentro deste escopo.

## Estado das entregas em 2026-09-17

Este registro representa o estado durante a revisão do
[PR #3](https://github.com/nickgrowthhack/clean-dev-cycle/pull/3). A CLI permanece
em andamento até sua integração ser comprovada. A atualização para integrada
acompanhará o próximo PR do escopo, com o SHA efetivo do merge.

Todos os responsáveis são `nick`. Branches de entregas planejadas serão criadas
quando o trabalho começar. As branches de validação e CI já existem e aguardam
a integração de seus pré-requisitos. Compartilhar o escopo não exige juntar
entregas num único PR.

| Entrega | Dependências | Estado | Branch | PR |
| --- | --- | --- | --- | --- |
| `branching-strategy` | Nenhuma | integrada | `nick/initial-cycle/branching-strategy` | [#2](https://github.com/nickgrowthhack/clean-dev-cycle/pull/2) |
| `commit-cli` | Nenhuma | em andamento | `nick/initial-cycle/commit-cli` | [#3](https://github.com/nickgrowthhack/clean-dev-cycle/pull/3) |
| `changelog` | `commit-cli` | planejada | `nick/initial-cycle/changelog` | Ainda não aberto |
| `commit-validation` | `changelog` | bloqueada | `nick/initial-cycle/commit-validation` | Ainda não aberto |
| `ci-delivery` | `commit-validation` | bloqueada | `nick/initial-cycle/ci-delivery` | Ainda não aberto |
| `main-protection` | `ci-delivery` | planejada | `nick/initial-cycle/main-protection` | Ainda não aberto |
| `release-cycle` | `main-protection` | planejada | `nick/initial-cycle/release-cycle` | Ainda não aberto |

`release-cycle` registra uma capacidade futura. Seu detalhamento poderá gerar
entregas menores antes de criar a branch. Atualize dependências e critérios no
plano ao fazer essa divisão.

### Resultados, evidências e critérios de aceite

- **branching-strategy:** guia, plano e links no README integrados pelo PR #2,
  no commit `6712b01fb3b75dbdbfd39d04671805a097561344`. Foram verificados links,
  nomes, sintaxe dos exemplos PowerShell, commits, changelog e check-ci local.
  A árvore integrada foi comparada à entrega validada. A sintaxe dos exemplos
  foi conferida sem executar seus comandos de rebase ou push.
- **commit-cli:** geração, revisão e criação segura de commits, preservando stage
  parcial e hooks. A implementação deriva de `73a5cd8`, com autoria e mensagem
  preservadas. O PR #3 continua a entrega do PR #1. Aceite: código, testes,
  dependências e workflow iguais à implementação original, README com o guia,
  notas dos PRs preservadas e validações locais e remotas sobre a base atual.
- **changelog:** síntese por PR, preservação de notas anteriores e verificação
  de atualidade sem IA. Aceite: testes de geração, atualização, preservação e
  `--check`, além da nota do próprio PR. Fonte: incremento de `573c579` sobre
  `fca7985`, disponível na branch `nick/initial-cycle/commit-validation`.
- **commit-validation:** política compartilhada para mensagens e intervalos de
  commits. Fonte: incremento de `49013d4` sobre `573c579`. Está bloqueada até a
  integração do changelog. Aceite: testes da configuração, limites e mensagens,
  preservando a indicação de incompatibilidade da entrega.
- **ci-delivery:** validação de commits, título e changelog usando eventos de
  push e PR. Fonte: incremento de `518d333` sobre `49013d4`, acompanhado do ajuste
  documental `b70fe11`. Está bloqueada até a integração da validação de commits.
  Aceite: testes de eventos e checks no PR incremental, com base atualizada e
  checkout real, incluindo a documentação de adoção do CI.
- **main-protection:** aplicar as regras do guia e documentar sua configuração.
  Aceite: leitura das configurações remotas e evidência de bloqueio dos casos
  inválidos, sem experimentar pushes destrutivos na `main`. Ainda não executada.
- **release-cycle:** versão, tag e GitHub Release correspondentes às entregas,
  preservando as notas por PR. Aceite: ciclo real e retomada documentados, com
  links para versão, tag, release e execuções. Ainda não implementada. Seu plano
  deve definir o contrato com o CI antes da implementação.

A geração das notas e as verificações ainda não incluídas na CLI inicial usam
`clean-dev-cycle 0.1.0`, compilado da revisão
`518d3338e0a5699a0f310604e951378cd505373e` no checkout de desenvolvimento. Esse
binário é uma ferramenta de apoio, não faz parte do diff da CLI inicial e não
antecipa a integração das capacidades de changelog ou validação.

### Proteções observadas e política alvo

Na consulta ao GitHub nesta data, a `main` não tinha proteção tradicional nem
regras efetivas. Rebase, squash e merge commit estavam permitidos. Auto-merge e
exclusão automática de branches estavam desativados. Confirme novamente o estado
remoto antes de qualquer alteração de configuração.

A política alvo ainda será aplicada em `main-protection`. O PR #3 contém o
workflow de qualidade da CLI para Linux e Windows. O check `Entrega do PR`
pertence à entrega `ci-delivery` e ainda não está integrado. Os resultados de
check-ci executado como ferramenta de apoio são locais, distintos dos checks
executados pelo GitHub.

## Integração incremental

1. **Concluir a CLI pelo PR #3.** Preservar o guia, os links no README e a nota
   do PR #2. Gerar a síntese do PR #3 sobre a base atual, conferir todos os checks
   e integrar por rebase, exigindo o SHA validado.
2. **Extrair o changelog.** Depois da CLI, criar `nick/initial-cycle/changelog`
   sobre a `main` atualizada e reaplicar somente `573c579`. Preservar as notas já
   integradas ao resolver a criação do changelog. Não reaplicar a implementação
   da CLI contida nos ancestrais desse commit.
3. **Atualizar a validação.** Depois do changelog, reutilizar
   `nick/initial-cycle/commit-validation`, reaplicando somente `49013d4` sobre a
   `main` atualizada. O limite anterior dos commits exclusivos é `573c579`.
   Conferir e ajustar as mensagens sem alterar autoria, significado ou trailers.
4. **Atualizar o CI.** Depois da validação, reutilizar
   `nick/initial-cycle/ci-delivery`, reaplicando `518d333` e `b70fe11` sobre a
   `main` atualizada. O limite anterior é `49013d4`. Incluir o ajuste documental
   junto com a implementação e conferir o contrato do validador no PR real.
5. **Ativar as proteções.** Com o CI integrado e comprovado, exigir `Qualidade`
   e `Entrega do PR`, PR obrigatório, base atualizada e histórico linear.
   Habilitar somente rebase merge, aplicar a proteção também a administradores
   e bloquear force-push e exclusão da `main`.
6. **Encerrar as entregas concluídas.** Registrar PRs e evidências, retirar as
   dependências e só então excluir branches que deixaram de ser necessárias.

Antes de cada operação, conferir as pontas locais e remotas. Nos transplantes,
usar o limite anterior registrado para excluir os incrementos já entregues.
Conferir `git range-diff`, o diff final e os testes. Em branches publicadas,
coordenar a atualização e usar lease explícito sobre a ponta remota conferida.
Se ela mudar, revisar o trabalho novo antes de publicar.

As mensagens reaplicadas precisam passar na política atual, inclusive as linhas
do corpo e os trailers. Cada diff de changelog precisa respeitar o limite de
128 KiB, excluindo o próprio changelog. Se exceder o limite, dividir novamente
por resultado seguro, sem aumentar ou contornar o limite silenciosamente.

Commitar as mudanças de código e documentação antes de gerar a nota de cada PR,
usando seu número real e sua base atual. Preservar todas as entradas integradas
anteriormente e repetir `changelog --check` depois de qualquer atualização.

Na entrada de `ci-delivery`, a `main` já conterá seus pré-requisitos. Exigir o
check remoto antes dessa comprovação pode bloquear a adoção. PRs de release
ainda não têm dispensa no validador: o contrato será tratado em `release-cycle`.

## Validação das entregas

Para cada entrega de código extraída ou atualizada, executar novamente:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
git diff --check
```

Conferir também mensagens, nota do PR, execução do binário e checks no GitHub.
Não reutilizar resultados de outra base como prova da nova entrega. Falhas de
ambiente ficam registradas como validação pendente até a execução adequada.

| Cenário | Resultado esperado |
| --- | --- |
| Documentação e código no mesmo escopo | A documentação integra sem herdar a implementação |
| B depende de A ainda em revisão | O PR de B mostra somente B e aguarda A entrar na `main` |
| A entra por rebase merge | B usa o limite anterior registrado, sem reaplicar A, e passa novamente pelos checks |
| A ou `main` avança | Atualizar a base, revisar o diff e conferir a nota antes de integrar |
| Conflito de código ou changelog | Resolver e validar novamente ou abortar e registrar o bloqueio, preservando outras notas |
| A é cancelada | Revisar e bloquear, cancelar ou adaptar B antes de remover sua dependência |
| Funcionalidade integrada desativada | Comportamento anterior preservado, estado desativado testado e ativação registrada |
| Branch publicada muda durante reescrita | O lease recusa o push e exige nova coordenação |
| Entrega integrada | Incremento sem duplicação, mensagens válidas, notas preservadas e resultado conferido |

Automação de branches, suporte simultâneo a versões antigas e fila de integração
ficam fora do critério mínimo deste primeiro ciclo. A fila exige antes suporte
aos seus eventos, incluindo `merge_group`, e validação dos checks e da política
de merge. Decisões futuras devem preservar a divisão em entregas e a visibilidade
do que está resolvido, verificado ou pendente.
