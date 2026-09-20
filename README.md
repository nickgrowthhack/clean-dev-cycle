# clean-dev-cycle

Uma CLI para concluir mudanças pequenas, verificá-las e publicá-las na main.
Jujutsu organiza o trabalho local. A única branch é `main`. Serve a qualquer
repositório, em qualquer linguagem: os checks são comandos do próprio projeto
e a versão das releases não depende de arquivos de pacote.

```text
mudança no jj → commit com versão e notas → submit → checks locais → main → CI → release
```

Uma mudança deve resolver uma parte compreensível do problema e manter o projeto
funcionando. Não é necessário terminar uma funcionalidade inteira para integrar.
O [guia do ciclo](docs/CYCLE.md) resume o caminho diário e a recuperação de falhas.

## Preparar o ambiente

Instale Git e [Jujutsu 0.45.1](https://github.com/jj-vcs/jj/releases/tag/v0.45.1).
Os executáveis devem estar no PATH. Codex CLI autenticado é necessário apenas
para gerar mensagens ou notas com IA. Para preparar releases, instale também a
[GitHub CLI](https://cli.github.com/) e autentique com `gh auth login`. O remoto
`origin` deve apontar para o GitHub.

Para instalar a CLI, baixe o arquivo para Linux ou Windows x86_64 na
[página de Releases](https://github.com/nickgrowthhack/clean-dev-cycle/releases),
confira o checksum com o `.sha256` correspondente e coloque o binário no PATH.
Cada release anexa o binário e o checksum, então nenhum toolchain de compilação
é necessário no projeto que usa a CLI. Quem tem Rust instalado pode compilar a
partir do código com `cargo install --path . --locked`.

```sh
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
Com releases ativadas, a CLI calcula a versão e propõe as notas após a revisão da
mensagem. Você pode revisar e editar as notas antes de confirmar a entrega.
Confirmar inclui os arquivos da release, descreve a mudança e abre a próxima,
como `jj commit`. Cancelar qualquer etapa mantém a mudança em edição.
O comando preserva o conteúdo revisado. Se o workspace mudar durante a geração,
ele recusa a confirmação. Edições posteriores ao snapshot final ficam na próxima
mudança, sem entrar silenciosamente no commit revisado.

Para escrever a mensagem sem IA:

```sh
clean-dev-cycle commit --message "fix: corrigir a leitura do arquivo" --entry-file nota.md
clean-dev-cycle submit
```

`submit` atualiza `origin`, valida a mudança concluída e sua preparação de release
em `@-` e executa os checks
configurados no próprio commit, em uma cópia temporária. Se todos passarem,
publica exatamente esse SHA diretamente na `main`. O CI verifica Linux e Windows
após o push. Você pode continuar editando a próxima mudança durante os checks.

Para escolher uma camada anterior do stack:

```sh
clean-dev-cycle submit --revision ID_DA_MUDANCA
```

A camada precisa ser filha direta de `main@origin`. Se a base avançar ou a mudança
for reescrita durante os checks, o envio é recusado.
Reenviar um SHA já integrado informa sucesso sem repetir os checks ou o push.
Cada push tem sua execução de CI, visível na
[página de Actions](https://github.com/nickgrowthhack/clean-dev-cycle/actions).

## Configurar os checks locais

Versione `clean-dev-cycle.toml` na raiz do projeto. Cada lista contém um programa
e seus argumentos, executados em ordem, sem shell implícito. Use os comandos que
o projeto já tem, em qualquer linguagem:

```toml
[checks]
commands = [
    ["npm", "ci"],
    ["npm", "test"],
    ["pwsh", "-NoProfile", "-File", "scripts/check.ps1"],
]
```

Caminhos como `./scripts/check` são relativos à cópia do commit. Este repositório,
por ser escrito em Rust, usa `cargo fmt`, `cargo clippy` e `cargo test`; veja o
`clean-dev-cycle.toml` na raiz.

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

A integração com Codex mantém modelo e esforço de raciocínio da configuração
pessoal. A geração ocorre em diretório temporário, com ferramentas desativadas.
Uma mensagem inválida recebe no máximo uma tentativa de correção.

## Validar mensagens sem IA

```sh
clean-dev-cycle check-commit --message-file mensagem.txt
clean-dev-cycle check-commit --from origin/main~1 --to origin/main
```

O perfil é Conventional Commits, com título de até 100 caracteres
Unicode e mensagem de até 16 KiB. Corpo longo, maiúsculas e ponto final são
permitidos. O modo arquivo funciona sem repositório. Intervalos usam referências
Git e exigem histórico completo. Códigos de saída: `0` sucesso, `1` falha e `2` uso inválido.

## Comunicar uma entrega

O comando avulso `changelog` é independente da preparação de releases. Escolha
o intervalo Git que representa o resultado a comunicar:

```sh
clean-dev-cycle changelog --from origin/main~1 --to origin/main
clean-dev-cycle changelog --from origin/main~1 --to origin/main --entry-file nota.md
```

O primeiro comando sintetiza o diff acumulado com IA. O segundo valida uma nota
manual com título `###`, linha em branco e síntese, aceitando também entregas
com binários ou diffs grandes. Ambos emitem somente Markdown em stdout.
`--from` deve ser ancestral de `--to`.

## Automatizar versões e releases

Ative por projeto em `clean-dev-cycle.toml`:

```toml
[release]
enabled = true
```

Funciona em qualquer repositório com `origin` no github.com, independentemente da
linguagem. A versão vive em três lugares que a CLI mantém: o manifesto
`.clean-dev-cycle-release.json`, o `CHANGELOG.md` e a tag `vVERSÃO`. A CLI não
edita arquivos de pacote como `Cargo.toml`, `package.json` ou `pyproject.toml`.
Um projeto que precise da versão nesses arquivos lê o manifesto por conta própria.
Versões são estáveis a partir de `0.1.0`. Prereleases, repositórios com várias
versões e deploy ficam fora deste fluxo. Sem `[release]`, `commit` conclui a
mudança apenas com a mensagem, e `submit` e o CI não exigem preparação de release.

`commit` reserva a versão no próprio commit. `feat` incrementa minor, `fix` e
`perf` incrementam patch. Uma quebra (`!` ou `BREAKING CHANGE`) incrementa minor
em `0.x` e major a partir de `1.0`. Os demais tipos não iniciam releases, salvo
quebra explícita. A IA redige as notas, mas não escolhe a versão.

A primeira preparação usa `initial_version`, com padrão `0.1.0`:

```toml
[release]
enabled = true
initial_version = "1.4.0"
```

Um repositório que já publica tags `vVERSÃO` no GitHub continua a sequência a
partir da última release publicada, sem precisar de `initial_version`. As
preparações seguintes partem da versão no pai da mudança, inclusive em stacks
ainda não publicados. Reabrir a mesma mudança não incrementa novamente. É possível
continuar trabalhando durante o CI. Uma versão cujo CI falhou pode nunca ganhar
tag, deixando uma lacuna.

As notas sintetizam o diff acumulado desde a última release publicada ancestral,
consultada pelo `gh` durante a preparação. O intervalo fica fixado nos metadados.
Notas de versões preparadas enquanto outra release está pendente podem incluir
mudanças em comum. O changelog registra preparações, enquanto as
[GitHub Releases](https://github.com/nickgrowthhack/clean-dev-cycle/releases)
confirmam o que foi publicado. Texto do `CHANGELOG.md` fora dos marcadores de
release é mantido após as entradas.

Mensagem, versão, notas e conteúdo revisado são vinculados pelo manifesto
`.clean-dev-cycle-release.json`. A preparação usa um índice Git e operações
Jujutsu isolados, integrados somente após a confirmação. `commit --dry-run`
mostra a proposta completa sem alterar os arquivos ou concluir a mudança.
`--yes` confirma mensagem e notas sem interação.

Para notas manuais, use título `###`, linha em branco e síntese. `--entry-file`
aceita entregas com binários ou diffs que excedem 128 KiB. Combine com
`--message` para dispensar completamente a IA. Nenhum diff é truncado.

Rebase ou edição posterior invalida a preparação. Um `jj commit` direto
continua possível para mudanças internas. Uma mudança elegível sem preparação
é recusada pelo `submit` e pelo CI.

```sh
clean-dev-cycle release check --revision SHA
```

Essa verificação é determinística, sem IA ou consultas ao GitHub. O workflow
executa o comando no SHA do evento em Linux e Windows. Após os dois passarem,
o job `Release` executa `release publish --revision SHA` com `GITHUB_TOKEN`
e permissão `contents: write`, criando `vVERSÃO` e as mesmas notas no GitHub.
Cada `--asset CAMINHO` anexa um arquivo à release. O runner não precisa de
autenticação da IA. Nenhum commit adicional é criado.

Repositórios consumidores devem instalar a CLI no runner, por exemplo baixando
o binário da release e conferindo o `.sha256`, e adotar a mesma dependência
entre jobs: publicação somente após todos os checks, checkout do SHA do evento,
histórico completo e fila de publicação com `queue: max`, sem cancelamento.
O comando de publicação exige o contexto do GitHub Actions na main.

Se a publicação falhar, reexecute o job `Release` da execução original. Uma tag
já criada no SHA esperado é reutilizada, uma release correspondente não é
duplicada e assets já enviados não são reenviados. Tag em outro SHA, notas
divergentes ou asset com outro tamanho causam erro sem sobrescrita.
Publicações atrasadas não substituem uma versão maior como `Latest`.
Nunca mova uma tag já publicada.

## Desenvolvimento

Esta seção é sobre desenvolver a própria CLI, que é escrita em Rust. Use o
toolchain 1.98.1 indicado em `rust-toolchain.toml`.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Os testes usam Jujutsu real, remotos Git temporários e um provedor de IA simulado.
Não acessam a conta do Codex nem o histórico deste repositório.
Windows usa Rust GNU, com `as`, `dlltool` e as DLLs do Git no PATH.
`cargo build --release --locked` gera o binário otimizado quando necessário.
`--version` reflete a versão do manifesto de release, lida em `build.rs`; a
versão em `Cargo.toml` fica em `0.0.0` e não é usada.

O CI executa formatação e Clippy no Linux e testes/execução da CLI nos dois
sistemas, em cada push na main. Valida as mensagens de todos os commits do evento.
A execução manual valida a mensagem do commit selecionado contra seu pai.
Após a validação, o job `Binário` compila o binário otimizado em Linux e Windows
e o empacota com `.github/scripts/package.ps1`, gerando arquivo e `.sha256`.
O job `Release` publica a tag, as notas e anexa esses arquivos.
O histórico é linear, com force-push e exclusão da main bloqueados, inclusive
para administradores. O CI não é um requisito prévio para aceitar o push.

A CLI automatiza versão, changelog, tags, GitHub Releases e binários. Deploy é independente.
