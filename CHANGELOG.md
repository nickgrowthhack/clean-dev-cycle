# Changelog

Cada entrada resume a entrega completa de um pull request, revisada antes do merge.

## [Não lançado]

<!-- clean-dev-cycle:pr:3:start -->
<!-- clean-dev-cycle:fingerprint:78b9be0d85feffa79d1d587dbffdc90f12d2977f -->
### Geração e revisão de commits com Codex (#3)

Adiciona `clean-dev-cycle commit`, que usa o diff do stage para gerar mensagens em português do Brasil, valida Conventional Commits e permite confirmar, editar ou cancelar antes de criar o commit. Oferece simulação com `--dry-run`, confirmação sem interação com `--yes` e contexto adicional por arquivo. A simulação também usa IA e pode consumir cota.

O fluxo preserva a seleção parcial e executa os hooks existentes do Git, sem fazer push. Mudanças no stage, HEAD ou branch durante a revisão impedem o commit; alterações feitas pelos hooks permanecem disponíveis para inspeção. Pedidos de contexto, falhas de geração e mensagens inválidas também bloqueiam a criação.

Exige Git e Codex CLI autenticado com suporte a `--ignore-user-config` e `--output-schema`; a instalação pelo código usa Rust 1.98.1. A geração ocorre em diretório temporário, com ferramentas e plugins desativados. Aceita diff textual UTF-8 de até 128 KiB e contexto de até 16 KiB; recusa binários, submódulos, conflitos, operações Git em andamento e nomes comuns de arquivos sensíveis.

Inclui CI para Linux e Windows com verificações de formatação, análise estática, testes, compilação e execução das opções de ajuda e versão do binário.
<!-- clean-dev-cycle:pr:3:end -->

<!-- clean-dev-cycle:pr:2:start -->
<!-- clean-dev-cycle:fingerprint:aecda254f59ec32d4e175fb0ab80d166de4f560d -->
### Estratégia de branches e plano do ciclo inicial (#2)

Documenta o fluxo de entregas pequenas com integração na `main`, convenção de nomes, registro de dependências e revisão de PRs encadeados. Orienta a atualização de bases, a verificação do changelog e a recuperação de conflitos ou regressões.

Define o plano do ciclo inicial, com responsáveis, critérios de aceite e roteiro para migrar o trabalho acumulado em entregas separadas. A migração, as proteções da `main` e o ciclo de releases ficam planejados; esta entrega é exclusivamente documental.
<!-- clean-dev-cycle:pr:2:end -->
