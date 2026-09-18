# initial-cycle: primeiro ciclo completo de desenvolvimento

O objetivo é comprovar commit, validação, changelog, versão, tag e GitHub Release,
com entregas pequenas e retomada sem publicação duplicada. Seguimos o
[guia de branching](../branching.md). Estado e evidências ficam nos PRs.

## Decisões

- Um perfil fixo de commits: estrutura convencional, tipos conhecidos e título
  com até 100 caracteres Unicode. Sem limites por linha no corpo, regras de caixa,
  ponto final, configuração por projeto ou equivalência com commitlint.
- Commits significativos preservados por rebase. Título descritivo livre no PR.
- Nota obrigatória para PRs com `feat`, `fix`, `perf` ou qualquer incompatibilidade.
  Demais tipos podem incluir nota, mas não precisam. A nota sintetiza o PR completo.
- Geração por IA opcional, com alternativa manual verificável para o changelog.
- PRs concentram estado e evidências. Planos registram decisões e dependências.

## Entregas da simplificação

| Entrega | Dependência | Critério de conclusão |
| --- | --- | --- |
| Validação simples | CLI existente | `check-commit` valida arquivos e intervalos sem IA, configuração ou mutações, com o perfil da `main` |
| Changelog manual | Validação simples | Nota fornecida por arquivo e fingerprint de metadados permitem verificar binários e diffs grandes sem IA, preservando notas antigas |
| CI simplificado | Changelog manual | PR interno passa sem nota, PR relevante exige nota atual, título livre e commits validados |
| Proteção da main | CI simplificado | Checks comprovados e obrigatórios, PR, base atualizada e rebase, incluindo administrador, sem aprovação externa obrigatória |

Cada entrega parte da `main` atualizada e integra antes da próxima. Código,
testes e documentação são revisados juntos. Não há tabela de status a sincronizar.

Para cada alteração de código, executar formatação, Clippy, testes, build,
ajuda/versão e `git diff --check`. Conferir Linux e Windows no PR correspondente.
Registrar evidências no próprio PR. Não reutilizar verificações de outra revisão.

## Consulta excepcional ao trabalho anterior

As branches anteriores à simplificação ficam fora do fluxo ativo. Novas entregas
partem da `main`, que contém o método escolhido. O reaproveitamento seletivo foi
concluído. Não retome as branches antigas como entregas pendentes nem reaplique
a CLI e o changelog dos PRs #3 e #4.

- `49013d4`, após `573c579`: referência histórica da validação configurável.
  Sua política, fixtures de commitlint e anúncio de incompatibilidade foram
  descartados. Essa política nunca fez parte da `main`.
- `518d333` e `b70fe11`, após `49013d4`: referência histórica do CI anterior.
  A validação de pushes e a exigência de título convencional foram descartadas.
- Preservar checkouts, branches e notas anteriores. A exclusão física dos
  checkouts antigos fica fora desta correção.

Usar o binário compilado da própria entrega. A ferramenta de apoio antiga não
pode impor as regras abandonadas nem validar os fingerprints novos.
O registro local `.tools/integration/RETOMADA.md` orienta a operação deste workspace.
Ele não acompanha novos clones nem substitui os PRs como fonte de evidências.

## Próximo objetivo após a simplificação

Implementar e comprovar versão, tag e GitHub Release correspondentes às entregas,
com uma autoridade de versionamento e retomada sem publicação duplicada. Essa
entrega terá plano próprio para autenticação, contrato com CI e recuperação.

Novos provedores, coordenação automática de todo o fluxo, revisão por IA de outros
formatos e manutenção simultânea de versões ficam adiados.
