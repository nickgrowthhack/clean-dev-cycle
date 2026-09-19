# Changelog

<!-- clean-dev-cycle:release:start -->
## 0.1.0

### Ciclo de entrega com Jujutsu e publicação automática

A CLI reúne a conclusão de mudanças pequenas no Jujutsu, a validação de Conventional Commits e o envio direto para a main após checks locais em uma cópia isolada. Mensagens podem ser geradas com Codex ou fornecidas manualmente, com revisão antes da conclusão da mudança.

Projetos Rust com releases ativadas passam a incluir versão, notas e metadados de verificação no próprio commit. A versão segue regras determinísticas, enquanto as notas sintetizam o resultado acumulado desde a última release publicada. É possível revisar a proposta completa, fornecer notas manuais para binários ou diffs grandes e continuar preparando entregas enquanto o CI está em andamento.

Após aprovação do commit em Linux e Windows, o GitHub cria a tag e publica as mesmas notas como release. A publicação pode ser retomada após interrupção sem duplicar releases ou substituir tags. O changelog histórico foi recuperado e identificado como registro do fluxo anterior. Esta primeira versão oferece tag e notas, sem distribuição de binários ou deploy.
<!-- clean-dev-cycle:release:end -->

