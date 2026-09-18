# Branching por entregas pequenas

Usamos trunk-based: branches curtas partem da `main` e retornam por PR com rebase.
Um commit representa uma mudança significativa e concluída. Um PR pode reunir
vários commits coerentes com o mesmo resultado. O histórico da `main` é preservado.

## Caminho habitual

1. Defina o resultado e como verificá-lo. Conclua uma entrega antes de iniciar outra.
2. Parta da `main` remota atualizada e use `<autor>/<escopo>/<entrega>` como nome.
   Use minúsculas ASCII, números e hífens, sem hífens nas extremidades dos segmentos.
   Confira o nome com `git check-ref-format --branch NOME`.
3. Implemente código, testes e documentação da entrega. Selecione o stage com cuidado.
4. Crie commits compreensíveis e valide o intervalo com `check-commit`.
5. Abra o PR com título descritivo, resultado, validação e incompatibilidades.
6. Registre a nota da entrega quando houver `feat`, `fix`, `perf` ou incompatibilidade.
   Use o número real do PR. Para os demais tipos, a nota é opcional.
7. Se a base avançar, atualize por rebase, revise conflitos e repita os checks.
   Integre somente o SHA revisado e validado, sem bypass administrativo.

O PR é a fonte do estado, responsáveis, discussão e evidências da entrega.
O plano em `docs/plans/<escopo>.md` guarda objetivo, decisões, dependências e
critérios de conclusão. Não espelhe estados do GitHub nem crie commits para
registrar que outro PR foi integrado. Atualize o plano quando uma decisão mudar.

Tamanho do diff e duração do trabalho ajudam a identificar dificuldades, mas
não demonstram que uma entrega esteja completa ou incompleta. Os limites do
gerador por IA são técnicos. Não omita arquivos para produzir uma revisão parcial.

## Dependências reais

Compartilhar um escopo não implica dependência de código. Use worktrees separados
quando houver necessidade de manter checkouts simultâneos. Uma entrega dependente
pode ser revisada contra a branch predecessora, mas só integra na `main` após ela.

Registre no PR dependente o SHA da predecessora incluído em sua base. Depois da
integração por rebase, reaplique somente os commits exclusivos da dependente com
`git rebase --onto NOVA_BASE LIMITE_ANTIGO`. Confira `git range-diff`, mude a base do
PR para `main` e repita a validação. A revisão contra a predecessora nunca autoriza
integrar o trabalho dependente nela.

Antes de reescrever uma branch publicada, confira o trabalho remoto e coordene
com seus responsáveis. Use `--force-with-lease` com o SHA remoto conferido.
Se o lease falhar, inspecione o trabalho novo. Não use `--force`.

## Conflitos, recuperação e proteção

Em conflitos, preserve a intenção de ambas as mudanças e as notas existentes.
Se a intenção não estiver clara, aborte o rebase e peça esclarecimento. Use
correção ou revert por PR para regressões na `main`, sem reescrever seu histórico.

A política alvo exige PR, base atualizada, histórico linear e os checks
`Qualidade` e `Entrega do PR`, também para o administrador, sem force-push ou
exclusão da `main`. Para o fluxo individual, não exige aprovação de outra pessoa.
As regras remotas só serão ativadas depois de seus checks estarem integrados e
comprovados. A configuração dos workflows não ativa a proteção da branch.

O [plano inicial](plans/initial-cycle.md) separa a recuperação das branches antigas
do caminho cotidiano. Automação de branches, fila de integração e manutenção de
várias versões não fazem parte deste primeiro ciclo.
