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
não comprovam esse resultado. A implementação de versionamento e releases ainda
precisa de seu próprio plano e PR, dentro deste escopo.

## Inventário observado em 2026-09-17

| Referência | Conteúdo observado | Situação |
| --- | --- | --- |
| `main`, em `c3b6e51` | Apenas o README inicial | Nenhuma entrega de código integrada |
| `nick/integrate-commit-cli`, em `32b3198` | CLI inicial e sua nota de changelog | [PR #1](https://github.com/nickgrowthhack/clean-dev-cycle/pull/1) aberto para `main` |
| `nick/outline`, em `49013d4` | CLI, changelog e validação de commits acumulados | Fonte para extrair entregas, sem PR aberto |
| `nick/ci-delivery`, em `518d333` | Incremento de CI sobre `nick/outline` | Fonte para extrair a entrega de CI, sem PR aberto |
| `nick/initial-cycle/branching-strategy` | Guia, este plano e links no README, partindo de `main` | Entrega documental independente, PR ainda não aberto |

Os commits `fca7985` e `73a5cd8` têm a mesma árvore de arquivos. O segundo ajusta
a mensagem da implementação inicial para a integração. Os SHAs diferentes não
representam duas entregas de código. A branch do PR #1 inclui também `32b3198`,
que registra sua nota de changelog.

O checkout de trabalho anterior permanece em `nick/ci-delivery`. A documentação
é preparada em um worktree separado na branch `nick/initial-cycle/branching-strategy`.

### Proteções verificadas e política alvo

A consulta ao GitHub nesta data encontrou `main` com `protected: false`, proteção
tradicional desativada e nenhuma regra retornada para essa branch. Rebase,
squash e merge commit estão permitidos. Auto-merge e exclusão automática de
branches estão desativados. Esses dados são um retrato da consulta, não uma
garantia sobre o estado futuro.

A política alvo do guia ainda precisa ser aplicada na entrega `main-protection`.
O workflow de `nick/ci-delivery` implementa verificações de entrega, mas não está
integrado na `main`. Antes de migrar, consulte novamente branches, PRs, checks e
proteções. Não trate este inventário como autorização para sobrescrever trabalho
que tenha avançado depois dele.

## Entregas e dependências

Todos os responsáveis abaixo são `nick`. `Planejada` indica que a branch futura
ainda não foi criada. As branches existentes mantêm seus nomes até a migração.

| Entrega | Dependências | Estado | Branch de entrega | PR |
| --- | --- | --- | --- | --- |
| `branching-strategy` | Nenhuma | em andamento | `nick/initial-cycle/branching-strategy` | Ainda não aberto |
| `commit-cli` | Nenhuma | em andamento | `nick/integrate-commit-cli` | [#1](https://github.com/nickgrowthhack/clean-dev-cycle/pull/1) |
| `changelog` | `commit-cli` | planejada | `nick/initial-cycle/changelog` | Ainda não aberto |
| `commit-validation` | `changelog` | planejada | `nick/initial-cycle/commit-validation` | Ainda não aberto |
| `ci-delivery` | `commit-validation` | planejada | `nick/initial-cycle/ci-delivery` | Ainda não aberto |
| `main-protection` | `ci-delivery` | planejada | `nick/initial-cycle/main-protection` | Ainda não aberto |
| `release-cycle` | `main-protection` | planejada | `nick/initial-cycle/release-cycle` | Ainda não aberto |

Essa ordem reflete as dependências do código acumulado existente. A estratégia
documental pode integrar a qualquer momento, independentemente da sequência.
`release-cycle` registra uma capacidade futura: seu detalhamento poderá gerar
entregas menores antes de criar a branch. Atualize dependências e critérios no
plano ao fazer essa divisão.

### Resultado e validação de cada entrega

- **branching-strategy:** guia aplicável a entregas independentes e encadeadas,
  convenção definida e migração rastreável. Aceite: links locais válidos, exemplos
  coerentes, nomes aceitos pelo Git e diff exclusivamente documental. Registrar
  o PR e a evidência de revisão antes de marcar como integrada.
- **commit-cli:** geração, revisão e criação segura de commits, preservando stage
  parcial e hooks. Aceite: revisar o diff do PR #1, sua nota e os checks de
  qualidade sobre a base atual. Evidência atual: implementação e PR existentes,
  ainda sem integração na `main`.
- **changelog:** síntese por PR, preservação de notas anteriores e verificação
  de atualidade sem IA. Aceite: testes de geração, atualização, preservação e
  `--check`, além da nota do próprio PR. Evidência atual: código em `573c579`,
  ainda sem validação da extração.
- **commit-validation:** política compartilhada para mensagens e intervalos de
  commits. Aceite: testes da configuração, limites, mensagens válidas e inválidas,
  preservando a indicação de incompatibilidade da entrega. Evidência atual:
  código em `49013d4`, ainda sem validação da extração.
- **ci-delivery:** validar commits, título e changelog usando os eventos reais de
  push e PR. Aceite: testes de eventos e checks no PR incremental, com base
  atualizada e checkout real. Evidência atual: código em `518d333`, ainda sem
  validação da extração.
- **main-protection:** aplicar as regras do guia e documentar sua configuração.
  Aceite: leitura das configurações remotas e evidência de bloqueio dos casos
  inválidos, sem experimentar pushes destrutivos na `main`. Ainda não executada.
- **release-cycle:** versão, tag e GitHub Release correspondentes às entregas,
  preservando as notas por PR. Aceite: ciclo real e retomada documentados, com
  links para versão, tag, release e execuções. Ainda não implementada. O plano
  desta capacidade deve definir seu contrato com o CI antes de implementá-la.

Para cada entrega de código extraída, executar novamente:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
git diff --check
```

Conferir também o diff, commits, nota do PR e resultado dos checks no GitHub.

Não reutilizar resultados antigos como prova da nova base. Falhas de ambiente
ficam registradas como validação pendente até a execução em ambiente adequado.

## Migração, uma entrega por vez

Esta seção é um roteiro para execução posterior. A entrega documental não faz
merge, reescrita ou renome de branches existentes, nem altera proteções remotas.

1. **Integrar a CLI pelo PR #1.** Preservar o PR e sua branch. Conferir a base,
   os checks e a nota antes do rebase merge. Se a documentação entrar primeiro,
   preservar seus links ao resolver o README. A integração gera novos SHAs.
2. **Extrair o changelog sobre a nova `main`.** Criar
   `nick/initial-cycle/changelog` e reaplicar apenas o incremento de `573c579`
   sobre `fca7985`. Não reaplicar a CLI já entregue. Preservar a nota do PR #1
   quando resolver a criação de `CHANGELOG.md` nas duas linhas de histórico.
3. **Extrair a validação.** Após integrar o changelog, criar
   `nick/initial-cycle/commit-validation` sobre `main` e reaplicar apenas o
   incremento de `49013d4` sobre `573c579`.
4. **Extrair o CI.** Após integrar a validação, criar
   `nick/initial-cycle/ci-delivery` sobre `main` e reaplicar apenas o incremento
   de `518d333` sobre `49013d4`. Conferir os requisitos do validador no PR real.
5. **Ativar as proteções em entrega própria.** Com o CI integrado e comprovado,
   exigir `Qualidade` e `Entrega do PR`, PR obrigatório, base atualizada e
   histórico linear. Habilitar somente rebase merge, aplicar a proteção também
   a administradores e bloquear force-push e exclusão da `main`.
6. **Conferir e encerrar as fontes antigas.** Comparar cada incremento migrado,
   documentar PRs e evidências, retirar dependências e só então encerrar as
   branches substituídas. Não excluir fontes durante a extração.

Antes de cada extração, conferir os SHAs e preservar as fontes. Aplicar o
incremento em uma branch nova, usando cherry-pick para preservar autoria. Ajustar
as mensagens incompatíveis na cópia extraída, preservando significado, trailers
e a indicação de breaking change. Não reescrever as branches antigas nem a
`main`. Usar `git range-diff` para revisar o que mudou no transplante, juntamente
com o diff final e os testes.

O diff acumulado de `nick/outline` para a `main` inicial ultrapassa o limite de
128 KiB da revisão de changelog. As mensagens antigas também precisam ser
conferidas contra a política de tamanho de linhas. A divisão resolve a unidade
de revisão, mas cada novo diff ainda precisa ser medido e validado. Se exceder o
limite, dividir novamente por resultado seguro, sem aumentar ou contornar o
limite silenciosamente.

Quando uma extração estiver validada, abrir seu PR, gerar a nota com o número
real e a base correta, incluir a nota e repetir `changelog --check`. Preservar
todas as entradas integradas anteriormente. Se a ferramenta ainda não estiver
disponível nessa base, usar um binário verificado de uma revisão que já a contém,
registrando sua versão e revisão, sem incluir código alheio na entrega.

Na entrada de `ci-delivery`, a `main` já conterá os pré-requisitos. Assim o PR
incremental poderá passar pelo próprio validador. Exigir o novo check na `main`
antes dessa comprovação pode bloquear o bootstrap. PRs de release ainda não têm
dispensa no validador atual: o contrato será tratado em `release-cycle`.

## Cenários de aceite da estratégia

| Cenário | Resultado esperado |
| --- | --- |
| Documentação e código no mesmo escopo | A documentação integra sem herdar a implementação |
| B depende de A ainda em revisão | O PR de B mostra somente B e aguarda A entrar na `main` |
| A entra por rebase merge | B usa o limite antigo registrado, sem reaplicar A, e passa novamente pelos checks |
| A ou `main` avança | Atualizar a base, revisar o diff e conferir a nota antes de integrar |
| Conflito de código ou changelog | Resolver e validar novamente ou abortar e registrar o bloqueio, preservando outras notas |
| A é cancelada | Revisar e bloquear, cancelar ou adaptar B antes de remover sua dependência |
| Funcionalidade integrada desativada | Comportamento anterior preservado, estado desativado testado e ativação registrada |
| Branch publicada muda durante reescrita | O lease recusa o push e exige nova coordenação |
| Migração concluída | Incrementos sem duplicação, mensagens válidas, notas preservadas e fontes conferidas |
| Documentação entregue | Guia, plano e links no README, sem alterações na CLI ou nas proteções |

## Trabalho posterior

Automação de branches, suporte simultâneo a versões antigas e fila de integração
ficam fora da entrega documental e do critério mínimo deste primeiro ciclo. A
fila exige antes suporte aos seus eventos, incluindo `merge_group`, e validação
dos checks e da política de merge. Decisões futuras devem preservar a divisão
em entregas e a visibilidade do que está resolvido, verificado ou pendente.
