# initial-cycle: registro histórico

Este plano foi encerrado como orientação operacional em 18 de setembro de 2026.
O método vigente está no [fluxo com Jujutsu](../branching.md).

As decisões anteriores de PR obrigatório, changelog por PR, check `Entrega do PR`,
branches por entrega e rebase de integração foram substituídas por mudanças locais
do Jujutsu e um envio por vez. O fluxo atual verifica o commit localmente e o
publica diretamente na main, com CI após o push e sem branch intermediária.
Não retome as branches antigas como trabalho pendente.

Foram preservados o perfil simples de mensagens, a alternativa sem IA e o
histórico do changelog. Os commits e PRs anteriores continuam sendo evidência
do que foi feito, sem determinar o procedimento atual.

O antigo objetivo de automatizar versão, tag e GitHub Release não foi implementado
nesta migração. Ele permanece separado, para ser planejado quando necessário.
