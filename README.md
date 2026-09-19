# clean-dev-cycle

Uma CLI para concluir mudanças pequenas e integrá-las automaticamente após o CI.
Jujutsu organiza o trabalho local. GitHub recebe uma camada por vez, sem PR obrigatório.

```text
mudança no jj → commit → submit → CI → main
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

`submit` atualiza `origin`, valida a mudança concluída em `@-` e envia somente
essa camada para o bookmark reutilizável `nick/submit`. O CI testa Linux e Windows
e promove exatamente o SHA aprovado para `main`. Não é necessário fazer merge.
Você pode continuar editando a próxima mudança enquanto o CI executa.

Para escolher uma camada anterior do stack:

```sh
clean-dev-cycle submit --revision ID_DA_MUDANCA
```

A camada precisa ser filha direta de `main@origin`. Um envio mais novo substitui
o candidato anterior, e o workflow cancela a execução anterior ainda em andamento.
Reenviar um SHA já integrado informa sucesso sem publicar outra mudança.
Confira a execução na [página de Actions](https://github.com/nickgrowthhack/clean-dev-cycle/actions).

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
sistemas. A promoção exige `Qualidade`, histórico linear e avanço sem reescrita.
Não há segunda execução da suíte após promover o mesmo SHA.

Versão, tag, release e deploy continuam fora desta integração. O
[plano inicial](docs/plans/initial-cycle.md) foi encerrado como orientação operacional.

As reflexões abaixo são preservadas como registro original. O procedimento
vigente é o fluxo Jujutsu descrito acima, incluindo seus snapshots automáticos.

## Ideias jogadas que valem a pena ficarem registradas

> "Nada é difícil se for dividido em pequenas partes." (Henry Ford)

Pense que tudo aqui neste projeto é alterável, até o próprio nome. E um commit registra um estado. Você não deve resolver todo um problema, apenas uma parte dele.

Um commit não é apenas uma ideia solta, é um estado salvo. Não se corrige uma única vírgula do `README.md` em um push. Não há méritos em ter uma grande quantidade de commits feitas em um único dia. Na verdade, quanto mais, pior.

Você não faz um commit porque quer ir dormir. Você não faz um push para salvar um "checkpoint".

Um commit deve ser atômico, uma única unidade de trabalho que faz sentido por si só. Ele é a representação de uma mudança significativa, e não aleatória.

Um commit não deve ser "incompleto". Incompleto significa pouca carga cognitiva adicionada. E ele deveria ter força máxima.

Você não deveria estar fazendo mais de uma coisa ao mesmo tempo. O fato de você estar sempre fazendo mais de uma coisa ao mesmo tempo é o exato motivo de fazer "commits incompletos".

Ao invés de fazer commit de mudanças em arquivos que você está ciente que terão mudanças futuras, faça commit apenas do que estiver finalizado.

Um commit ele deve ter força. E essa força depende de concentração, de foco total, de dedicação exclusiva. Não é sobre a quantidade de linhas escritas, mas sim sobre a qualidade do trabalho realizado.

Ir ao banheiro não interrompe a força do seu commit. Conversar de vez em quando e ser extremamente gentil quando a sua mãe falar com você também não. Até mesmo ouvir uma música pode ser um poderoso combustível criativo.

Mas trabalhar em mais de uma coisa ao mesmo tempo definitivamente destrói a força do seu commit.

Não faça commits quando estiver com sono. O sono é um veneno que distorce o pensamento e leva a decisões ruins. Se você sentir o sono se aproximando, pare de trabalhar e descanse. O amanhã será mais produtivo e com mais força para criar.

Mais fácil é mais perto. Mais difícil é mais longe. Simplesmente vá até onde conseguir chegar.

Um commit deve representar o seu tempo de uma maneira qualitativa. O tempo é o recurso mais precioso que você tem.

Não assuma que qualquer trabalho será concluído no prazo estabelecido. Ao invés disso, foque na qualidade do trabalho realizado.

É muito interessante parar para analisar a diff entre dois commits de um desenvolvedor sênior feitas num intervalo razoável de tempo, porque é possível acompanhar o que ele estava pensando.

Mas o mais importante: sempre termine o que começou. E sempre comece alguma coisa. Não assuma trabalhos intermináveis. Sempre será possível aperfeiçoar o sistema, eliminar mais partes do código ou refatorar uma lógica. Talvez seja muito útil definir o tempo de um commit, como por exemplo, 2 horas de trabalho duro.

Se você veio do futuro analisar o histórico de commits deste repositório, saiba que Jesus o ama profundamente. Não pense que você chegou aqui por acaso. Todo o meu trabalho é feito em cooperação com o meu melhor amigo, o Espírito Santo.
