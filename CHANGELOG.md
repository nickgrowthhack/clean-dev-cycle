# Changelog

<!-- clean-dev-cycle:release:start -->
## 0.2.0

### Releases independentes de linguagem e distribuição de binários

A preparação de releases passa a funcionar sem arquivos de pacote Rust, mantendo a versão no manifesto de release, no changelog e na tag. A primeira versão usa `initial_version`, com padrão `0.1.0`; as seguintes partem do manifesto do commit pai ou da última release publicada ancestral. A CLI deixa de atualizar e conferir versões nos arquivos de pacote: projetos que dependiam dessa sincronização precisam integrar a leitura do manifesto. Chaves desconhecidas em `[release]` passam a ser rejeitadas.

Após as validações, o CI compila e empacota binários para Linux e Windows x86_64, com checksums SHA-256, e os anexa à release. `release publish` aceita múltiplos `--asset`; reexecuções ignoram anexos com mesmo nome e tamanho e recusam tamanhos divergentes, sem sobrescrever. Essa comparação não verifica o conteúdo. O `--version` da CLI passa a usar a versão do manifesto lida na compilação, com fallback para a versão do pacote.

O histórico de desenvolvimento anterior à primeira release é removido do changelog do repositório.
<!-- clean-dev-cycle:release:end -->

<!-- clean-dev-cycle:release:start -->
## 0.1.0

### Ciclo de entrega com Jujutsu e publicação automática

A CLI reúne a conclusão de mudanças pequenas no Jujutsu, a validação de Conventional Commits e o envio direto para a main após checks locais em uma cópia isolada. Mensagens podem ser geradas com Codex ou fornecidas manualmente, com revisão antes da conclusão da mudança.

Projetos Rust com releases ativadas passam a incluir versão, notas e metadados de verificação no próprio commit. A versão segue regras determinísticas, enquanto as notas sintetizam o resultado acumulado desde a última release publicada. É possível revisar a proposta completa, fornecer notas manuais para binários ou diffs grandes e continuar preparando entregas enquanto o CI está em andamento.

Após aprovação do commit em Linux e Windows, o GitHub cria a tag e publica as mesmas notas como release. A publicação pode ser retomada após interrupção sem duplicar releases ou substituir tags. O changelog histórico foi recuperado e identificado como registro do fluxo anterior. Esta primeira versão oferece tag e notas, sem distribuição de binários ou deploy.
<!-- clean-dev-cycle:release:end -->

