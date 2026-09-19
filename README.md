# clean-dev-cycle

Uma CLI para concluir mudanças pequenas, verificá-las e publicá-las na main.
Jujutsu organiza o trabalho local. A única branch é `main`, sem PR ou promoção.

```text
mudança no jj → commit → submit → checks locais → push na main → CI
```

Uma mudança deve resolver uma parte compreensível do problema e manter o projeto
funcionando. Não é necessário terminar uma funcionalidade inteira para integrar.
O [guia do fluxo](docs/branching.md) explica stacks, falhas e recuperação.

## Preparar o ambiente

Instale Git, [Jujutsu 0.45.1](https://github.com/jj-vcs/jj/releases/tag/v0.45.1)
e o toolchain Rust 1.98.1 indicado em `rust-toolchain.toml`.
Os executáveis devem estar no PATH. Codex CLI autenticado é necessário apenas
para gerar mensagens ou notas com IA.

```sh
cargo install --path . --locked
jj git init --colocate
jj config set --repo user.name "Seu nome"
jj config set --repo user.email "seu-email"
jj git fetch --remote origin
jj bookmark track main@origin
```

A CLI confere nome e e-mail antes de gerar a mensagem. Se uma mudança foi criada
antes de configurar a identidade, confira `jj config list user -T builtin_config_list_detailed`.
Para uma mudança sua com autor vazio, use `jj metaedit -r ID --update-author`.
Para corrigir apenas a identidade de quem registrou um commit já concluído, use
`jj metaedit -r ID --force-rewrite` e reenvie. O conteúdo e a mensagem são mantidos,
mas o SHA muda e precisa passar pelo CI.

Em um workspace sem trabalho em andamento, comece sobre a versão atual:

```sh
jj new main@origin
```

## Concluir e enviar uma mudança

Edite os arquivos e confira `jj diff`. Use `jj split` se houver mudanças
independentes. Para gerar uma descrição e confirmá-la:

```sh
clean-dev-cycle commit
clean-dev-cycle submit
```

`commit` revisa o diff de `@`. Enter confirma, `e` permite editar e `n` cancela.
Na edição, termine com uma linha contendo apenas `.` e confirme a nova mensagem.
Confirmar descreve a mudança e abre a próxima, como `jj commit`.
O comando preserva o conteúdo revisado. Se o workspace mudar durante a geração,
ele recusa a confirmação. Edições posteriores ao snapshot final ficam na próxima
mudança, sem entrar silenciosamente no commit revisado.

Para escrever a mensagem sem IA:

```sh
jj commit -m "fix: corrigir a leitura do arquivo"
clean-dev-cycle submit
```

`submit` atualiza `origin`, valida a mudança concluída em `@-` e executa os checks
configurados no próprio commit, em uma cópia temporária. Se todos passarem,
publica exatamente esse SHA diretamente na `main`. O CI verifica Linux e Windows
após o push. Você pode continuar editando a próxima mudança durante os checks.

Para escolher uma camada anterior do stack:

```sh
clean-dev-cycle submit --revision ID_DA_MUDANCA
```

A camada precisa ser filha direta de `main@origin`. Se a base avançar ou a mudança
for reescrita durante os checks, o envio é recusado. Faça rebase, revise e reenvie.
Reenviar um SHA já integrado informa sucesso sem repetir os checks ou o push.
Cada push tem sua execução de CI. Uma falha após publicação exige uma correção
ou reversão como novo commit na main, sem reescrever o histórico.
Confira a execução na [página de Actions](https://github.com/nickgrowthhack/clean-dev-cycle/actions).

## Configurar os checks locais

Versione `clean-dev-cycle.toml` na raiz do projeto. Neste repositório:

```toml
[checks]
commands = [
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings"],
    ["cargo", "test", "--locked"],
]
```

Cada lista contém um programa e seus argumentos, executados em ordem, sem shell
implícito. Outros projetos podem definir seus próprios comandos. Caminhos como
`./scripts/check` são relativos à cópia do commit. Para scripts PowerShell, use
uma lista como `["pwsh", "-NoProfile", "-File", "scripts/check.ps1"]`.

Os programas precisam estar instalados. A cópia contém os arquivos versionados,
sem dependências ou arquivos ignorados do workspace. Inclua a preparação necessária
nos comandos. A configuração controla os checks locais, não o workflow do GitHub.

Configuração ausente, inválida ou vazia impede o envio. Cada comando tem limite de
15 minutos. Erro, cancelamento, ferramenta ausente ou alteração de arquivos
versionados pelos checks impede o push. Use modos de verificação que não corrijam
arquivos automaticamente. Não há opção para pular os checks.

A cópia temporária usa HEAD destacado e é removida ao terminar. As edições da
próxima mudança não participam da verificação. Os checks usam a configuração
versionada no commit selecionado, mesmo que você já tenha editado a configuração
em `@`.

## Opções e limites da geração

`commit --dry-run` mostra a proposta sem descrever ou concluir a mudança.
`--yes` confirma sem interação. `--context-file ARQUIVO` acrescenta intenção em
UTF-8, até 16 KiB. Também há `--model`, `--codex` e `--timeout` (120 segundos).
A geração, inclusive em simulação, utiliza o Codex e pode consumir cota.
Comandos de leitura do Jujutsu podem salvar snapshots locais.

O diff enviado à IA deve ser textual, UTF-8 e ter até 128 KiB. Binários,
submódulos, nomes potencialmente sensíveis e diffs maiores são recusados.
Nenhuma parte é truncada. Para esses casos, revise o conteúdo e use `jj commit`.
O stage e os hooks do Git não participam do fluxo Jujutsu.

A integração com Codex mantém modelo e esforço de raciocínio da configuração
pessoal. A geração ocorre em diretório temporário, com ferramentas desativadas.
Uma mensagem inválida recebe no máximo uma tentativa de correção.

## Validar mensagens sem IA

```sh
clean-dev-cycle check-commit --message-file mensagem.txt
clean-dev-cycle check-commit --from origin/main~1 --to origin/main
```

O perfil continua sendo Conventional Commits, com título de até 100 caracteres
Unicode e mensagem de até 16 KiB. Corpo longo, maiúsculas e ponto final são
permitidos. O modo arquivo funciona sem repositório. Intervalos usam referências
Git e exigem histórico completo. Códigos de saída: `0` sucesso, `1` falha e `2` uso inválido.

## Comunicar uma entrega

O changelog é opcional e independente da integração. Escolha o intervalo Git
que representa o resultado a comunicar:

```sh
clean-dev-cycle changelog --from origin/main~1 --to origin/main
clean-dev-cycle changelog --from origin/main~1 --to origin/main --entry-file nota.md
```

O primeiro comando sintetiza o diff acumulado com IA. O segundo valida uma nota
manual com título `###`, linha em branco e síntese, aceitando também entregas
com binários ou diffs grandes. Ambos emitem somente Markdown em stdout.
Não escrevem em `CHANGELOG.md`. As notas históricas permanecem preservadas.
`--from` deve ser ancestral de `--to`. Não há número de PR, fingerprint ou
nota obrigatória para liberar uma integração.

## Desenvolvimento

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Os testes usam Jujutsu real, remotos Git temporários e um provedor de IA simulado.
Não acessam a conta do Codex nem o histórico deste repositório.
Windows usa Rust GNU, com `as`, `dlltool` e as DLLs do Git no PATH.
`cargo build --release --locked` gera o binário otimizado quando necessário.

O CI executa formatação e Clippy no Linux e testes/execução da CLI nos dois
sistemas, em cada push na main. Valida as mensagens de todos os commits do evento.
A execução manual valida a mensagem do commit selecionado contra seu pai.
O histórico é linear, com force-push e exclusão da main bloqueados, inclusive
para administradores. O CI não é um requisito prévio para aceitar o push.

Versão, tag, release e deploy continuam fora desta integração. O
[plano inicial](docs/plans/initial-cycle.md) foi encerrado como orientação operacional.
