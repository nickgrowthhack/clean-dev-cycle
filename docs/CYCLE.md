# Fluxo com Jujutsu e somente main

Trabalhe em uma mudança pequena por vez. Mantenha dependências em um stack local
e integre cada camada assim que ela estiver verificável. Jujutsu é a interface
de trabalho. O Git fornece armazenamento e transporte para GitHub.

## Caminho cotidiano

1. Edite a mudança atual (`@`) e confira `jj diff`.
2. Separe resultados independentes com `jj split`.
3. Conclua com `clean-dev-cycle commit`, revisando mensagem, versão e notas.
4. Execute `clean-dev-cycle submit`. O padrão é a mudança concluída em `@-`.
5. O submit verifica o commit e publica diretamente na main. Acompanhe o CI.

`submit --revision ID` envia uma camada anterior; a CLI indica qual vem primeiro.
Não envie `@`, que ainda está em edição. Os stacks são mudanças locais do Jujutsu,
sem bookmarks adicionais. As regras do `submit`, dos checks e das releases estão
no [README](../README.md).

## Quando algo falha

- Check local ou mensagem inválida: corrija a mudança com `jj edit ID`, confira o diff,
  conclua novamente e reenvie. Descendentes são reaplicados pelo Jujutsu.
- Base avançou: atualize o remoto e faça rebase da linha local com
  `jj git fetch --remote origin` e `jj rebase -b @ -o main@origin`. Resolva os
  conflitos, revise e reenvie. O novo SHA precisa de novos checks.
- Mudança reescrita durante os checks: revise a nova versão e execute submit
  novamente. O resultado anterior não é reaproveitado.
- Preparação de release desatualizada após edição ou rebase: reabra com `jj edit`
  e conclua novamente pela CLI. O submit não reescreve a mudança para corrigir notas.
- CI falhou ou houve regressão publicada: prepare uma correção ou reversão como
  nova mudança e envie pelo mesmo fluxo. Não reescreva a main.

Confira `jj op log` para investigar operações locais e `jj undo` para desfazer
a última operação apropriada. Uma operação local não desfaz uma integração
já publicada na main.
