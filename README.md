# clean-dev-cycle

CLI em Rust para automatizar commits e releases.

## Criar um commit

É necessário ter Git, Rust 1.98.1 e Codex CLI instalado e autenticado. Para instalar a versão deste checkout:

```sh
cargo install --path . --locked
```

Selecione as alterações que pertencem ao commit e execute:

```sh
git add caminho/do/arquivo
clean-dev-cycle commit
```

O comando envia o diff do stage ao Codex, valida a mensagem em Conventional Commits e exibe uma prévia em português do Brasil. Pressione Enter para confirmar, `e` para escrever uma nova mensagem ou `n` para cancelar. Na edição, termine com uma linha contendo apenas `.`. A mensagem editada também passa por validação e confirmação.

Para testar a geração sem criar um commit:

```sh
clean-dev-cycle commit --dry-run
```

`--dry-run` usa o Codex e pode consumir sua cota. Para fornecer a intenção da mudança, use um arquivo de texto UTF-8:

```sh
clean-dev-cycle commit --context-file contexto.txt
```

Se faltar contexto essencial, o comando informa a dúvida e encerra. Acrescente a explicação ao arquivo e execute novamente. O contexto é enviado junto com o diff; revise seu conteúdo e mantenha o arquivo fora do stage se ele não fizer parte da entrega.

Também estão disponíveis `--yes` para confirmar sem interação, `--model MODELO`, `--codex CAMINHO` e `--timeout SEGUNDOS` (120 por chamada). Consulte `clean-dev-cycle commit --help`.

O Codex é localizado primeiro no PATH. No Windows, se ele não estiver no PATH, a CLI procura o executável incluído no aplicativo Codex, em `%LOCALAPPDATA%/OpenAI/Codex/bin`. Se houver mais de uma versão nessa pasta, informe `--codex CAMINHO` para escolher. Um caminho explícito tem prioridade sobre a detecção automática.

O comando preserva a seleção parcial de arquivos e executa os hooks existentes do Git. Se o stage, HEAD ou a branch mudar durante a geração ou pelos hooks anteriores ao commit, a operação é recusada para permitir uma nova revisão. Alterações feitas pelos próprios hooks permanecem disponíveis para inspeção. O comando não faz push.

A integração usa a autenticação padrão OpenAI do Codex e reaproveita `model` e `model_reasoning_effort` do `config.toml`. As demais configurações pessoais, plugins e ferramentas ficam desativadas durante a geração, realizada em um diretório temporário separado. É necessária uma versão do Codex CLI com suporte a `exec --ignore-user-config` e `--output-schema`.

Nesta versão, o diff precisa ser textual, UTF-8 e ter até 128 KiB; arquivos binários, submódulos e operações de merge/rebase em andamento são recusados. O contexto adicional aceita até 16 KiB. Nomes comuns de arquivos sensíveis, como `.env` e chaves privadas, são bloqueados; isso não substitui a revisão do conteúdo selecionado. Uma resposta inválida do Codex recebe no máximo uma tentativa de correção.

## Desenvolvimento

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Por padrão, os testes usam um provedor simulado e repositórios temporários; não chamam a IA nem alteram o histórico deste repositório. O teste opcional com o Codex autenticado consome cota e pode ser executado com `cargo test --locked --test commit real_codex -- --ignored --nocapture`. Ele usa um diff de exemplo em um repositório temporário e não cria commits.

Para testar manualmente sem instalar, use `cargo run -- commit --dry-run` ou o binário em `target/release`.

No Windows, o toolchain GNU exige o MinGW-w64 com `as` e `dlltool` no PATH; o toolchain MSVC exige as ferramentas de compilação C++ e o Windows SDK. O CI verifica Windows com GNU e Linux.

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
