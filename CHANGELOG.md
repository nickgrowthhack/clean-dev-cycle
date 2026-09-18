# Changelog

Cada entrada resume a entrega completa de um pull request, revisada antes do merge.

## [Não lançado]

<!-- clean-dev-cycle:pr:6:start -->
<!-- clean-dev-cycle:fingerprint:v2:813a8ae03fc89b3cfb63c5c10a216e4322cdd2fc -->
### Changelog manual com verificação independente da IA (#6)

O comando `changelog` passa a aceitar uma síntese revisada por `--entry-file`, mantendo confirmação, edição, cancelamento e simulação. Esse caminho funciona sem IA para entregas com binários, conteúdo não UTF-8, submódulos e diffs acima do limite de geração. Também permite corrigir a redação de uma entrada quando o conteúdo do PR permanece igual.

As novas notas são vinculadas aos caminhos, modos e identificadores completos dos objetos Git alterados e ao contexto fornecido. A verificação de atualidade dispensa o patch textual e continua detectando mudanças posteriores. As notas anteriores permanecem intactas e verificáveis pelo formato original. A geração por IA mantém suas restrições e passa a informar a alternativa manual quando não puder concluir a revisão.
<!-- clean-dev-cycle:pr:6:end -->


<!-- clean-dev-cycle:pr:5:start -->
<!-- clean-dev-cycle:fingerprint:84b49b39d3c1833b8400751cac362fad31a70090 -->
### Validação de commits sem IA (#5)

Adiciona `check-commit` para validar mensagens em arquivos UTF-8 ou commits de um intervalo `FROM..TO`, sem IA nem alterações em arquivos, stage ou histórico. A validação por arquivo funciona sem Git e fora de repositórios, permitindo integração manual com hooks `commit-msg`. Usa o mesmo perfil Conventional Commits da geração, com título de até 100 caracteres Unicode e mensagem de até 16 KiB, sem configuração por projeto.

A validação de intervalos exige histórico completo, inclui commits de branches integradas e informa todas as mensagens inválidas. Intervalos vazios são aceitos. Os códigos de saída distinguem sucesso (`0`), erro de leitura ou validação (`1`) e uso inválido (`2`).

O fluxo documentado passa a concentrar estado e evidências nos PRs, mantendo decisões e dependências nos planos. Define nota de changelog obrigatória para entregas com `feat`, `fix`, `perf` ou quebra de compatibilidade, opcional nos demais casos e sempre referente ao PR completo. A aplicação dessa política pelo CI permanece prevista para uma entrega futura.
<!-- clean-dev-cycle:pr:5:end -->


<!-- clean-dev-cycle:pr:4:start -->
<!-- clean-dev-cycle:fingerprint:5f82fb2e99ec22c193c4e69eeeaf146bd92ebe2c -->
### Changelog com síntese da entrega completa por PR (#4)

A CLI passa a oferecer `changelog --base REF --pr NÚMERO`, que usa o Codex para propor uma síntese em português do diff acumulado do PR desde a base comum. A revisão considera apenas alterações commitadas, excluindo o próprio changelog, e permite confirmar, editar, cancelar ou visualizar a proposta com `--dry-run`.

O comando cria ou atualiza uma entrada por PR em `CHANGELOG.md`, preservando notas manuais, outros PRs e versões existentes. Novas entradas ficam em `Não lançado`; revisões substituem a entrada correspondente. A gravação é interrompida se detectar alterações concorrentes no repositório, nas referências, no contexto ou no changelog.

Com `--check`, é possível verificar se a entrada corresponde ao diff e ao contexto atuais, sem IA nem escrita. Repetir a geração com esses dados inalterados também dispensa nova chamada à IA. O fluxo exige referências locais atualizadas e histórico completo, recusa diffs incompletos ou incompatíveis com os limites documentados e deixa a inclusão da nota no commit a cargo de quem usa.
<!-- clean-dev-cycle:pr:4:end -->

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
