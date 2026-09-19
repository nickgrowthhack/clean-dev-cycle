# Fluxo com Jujutsu e somente main

Trabalhe em uma mudança pequena por vez. Mantenha dependências em um stack local
e integre cada camada assim que ela estiver verificável. Jujutsu é a interface
de trabalho. O Git fornece armazenamento e transporte para GitHub.

## Caminho cotidiano

1. Edite a mudança atual (`@`) e confira `jj diff`.
2. Separe resultados independentes com `jj split`.
3. Conclua com `clean-dev-cycle commit` ou `jj commit -m MENSAGEM`.
4. Execute `clean-dev-cycle submit`. O padrão é a mudança concluída em `@-`.
5. O submit verifica o commit e publica diretamente na main. Acompanhe o CI.

Use `submit --revision ID` para enviar uma camada anterior. A ferramenta indica
qual camada vem primeiro quando o stack ainda tem uma dependência não integrada.
Não envie `@`, que ainda está em edição. `main` é a única branch local e remota.
Os stacks são mudanças locais do Jujutsu, sem bookmarks adicionais.

## O que chega à main

O envio deve conter exatamente um commit filho da main atual, com alteração de
arquivos, mensagem e identidade válidas e sem conflitos. Antes do push, o submit
executa os comandos de `clean-dev-cycle.toml` do commit em um clone temporário,
com HEAD destacado. Falhas ou alterações de arquivos versionados pelos checks
impedem a publicação. Consulte a configuração e os limites no [README](../README.md).

O SHA aprovado localmente é enviado diretamente à main. O CI verifica Linux e
Windows depois da publicação. Um novo push não cancela a execução anterior.
Os checks locais são uma regra do submit e não substituem uma proteção no servidor
contra envios manuais.

Force-push, exclusão da main e histórico não linear são bloqueados,
inclusive para administradores. Changelog, release e deploy não bloqueiam o envio.

## Quando algo falha

- Check local ou mensagem inválida: corrija a mudança com `jj edit ID`, confira o diff,
  conclua novamente e reenvie. Descendentes são reaplicados pelo Jujutsu.
- Base avançou: atualize o remoto e faça rebase da linha local com
  `jj git fetch --remote origin` e `jj rebase -b @ -o main@origin`. Resolva os
  conflitos, revise e reenvie. O novo SHA precisa de novos checks.
- Mudança reescrita durante os checks: revise a nova versão e execute submit
  novamente. O resultado anterior não é reaproveitado.
- Mudança já integrada: `submit` informa isso e não repete checks nem push.
- CI falhou ou houve regressão publicada: prepare uma correção ou reversão como
  nova mudança e envie pelo mesmo fluxo. Não reescreva a main.

Confira `jj op log` para investigar operações locais e `jj undo` para desfazer
a última operação apropriada. Uma operação local não desfaz uma integração
já publicada na main.
