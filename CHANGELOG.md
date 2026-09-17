# Changelog

## [Não lançado]

<!-- clean-dev-cycle:pr:1:start -->
<!-- clean-dev-cycle:fingerprint:c6ca093485e50865da2c41a7ee80ed8115cdda37 -->
### Criação de commits com mensagens geradas pelo Codex (#1)

Adiciona `clean-dev-cycle commit`, que usa o diff do stage para gerar mensagens em português do Brasil e validá-las no padrão Conventional Commits. O fluxo permite confirmar, editar ou cancelar, simular a geração com `--dry-run`, confirmar sem interação com `--yes` e fornecer contexto com `--context-file`.

Preserva a seleção parcial de alterações e executa os hooks existentes do Git. O commit é recusado se o stage, HEAD ou a branch mudar durante a revisão, inclusive por ação dos hooks.

A integração exige Codex CLI autenticado e compatível com `--ignore-user-config` e `--output-schema`. Aceita diffs textuais UTF-8 de até 128 KiB e contexto de até 16 KiB. Recusa binários, submódulos, nomes comuns de arquivos sensíveis, conflitos e operações de merge, rebase, cherry-pick ou revert em andamento. Quando falta contexto essencial, informa a dúvida e encerra sem criar o commit.

Inclui CI para verificar formatação, análise estática, testes e compilação em Linux e Windows.
<!-- clean-dev-cycle:pr:1:end -->
