# Branching por entregas pequenas e escopos compartilhados

Adotamos trunk-based com branches curtas por entrega. A `main` é o ponto de
integração e deve permanecer utilizável. PRs encadeados permitem revisar partes
dependentes enquanto a predecessora aguarda integração.

O vínculo entre as partes fica em um plano versionado. Consulte o
[escopo initial-cycle](plans/initial-cycle.md) para a primeira aplicação e para o
estado observado das proteções do repositório. Este guia define a política
escolhida. Ele não instala proteções nem adiciona automação à CLI.

## Escopo, entrega e branch

Um **escopo** é um objetivo maior com critério de conclusão. Uma **entrega** é uma
parte concluível e verificável desse objetivo. Cada entrega corresponde a uma
branch e a um PR, que podem conter vários commits significativos.

Compartilhar um escopo não implica depender do código das outras entregas. Por
exemplo, documentar a estratégia e implementar o changelog pertencem ao ciclo
inicial, mas a documentação pode integrar antes do código.

O escopo não exige uma branch agregadora. Cada entrega retorna à `main`. Uma
branch dependente usa temporariamente outra branch como base de revisão, sem
transformá-la em um segundo ponto de integração.

### Convenção de nomes

```text
<autor>/<escopo>/<entrega>
nick/initial-cycle/branching-strategy
nick/initial-cycle/changelog
nick/initial-cycle/commit-validation
```

Use três segmentos em kebab-case: letras ASCII minúsculas e números, com palavras
separadas por hífens. Não use espaços, acentos ou hífens nas extremidades.
Valide o nome também com `git check-ref-format --branch NOME`.

`autor` identifica o responsável pela branch. `escopo` e `entrega` são
identificadores estáveis no plano. Não coloque ordem, estado ou nome da branch
predecessora no identificador. A `main` é a exceção à convenção.

Uma entrega não ganha outro identificador ao mudar de responsável. Registre a
transferência e a branch vigente no plano e coordene eventuais renomes com quem
trabalha nela. Antes de criar uma branch, confira se a entrega já está em curso.
As branches anteriores à adoção mantêm seus nomes durante a migração documentada.

### Registro mínimo

Mantenha um documento por escopo em `docs/plans/<escopo>.md`, com:

- Objetivo e critério verificável de conclusão do escopo.
- Identificador, resultado esperado e critério de aceite de cada entrega.
- Responsável, dependências por identificador, branch e PR de cada entrega.
- Estado, evidências de validação e bloqueios conhecidos.

Use `planejada`, `em andamento`, `bloqueada`, `integrada` ou `cancelada`. Código
pronto aguardando revisão ou merge continua **em andamento**. `Integrada` exige
entrada na `main` e evidência da validação. Registre o motivo e a condição de
retomada de um bloqueio. Uma entrega cancelada não satisfaz uma dependência.

Atualize o registro ao iniciar, bloquear, transferir, cancelar ou integrar uma
entrega. O PR referencia o documento e seu identificador de entrega. Ao integrar,
registre o link do PR e a revisão ou execução que comprova o resultado. Se isso
exigir uma atualização posterior do plano, faça uma pequena entrega documental
identificada no mesmo escopo.

## Quando integrar

Integre uma parte assim que seu resultado estiver concluído, revisado e validado,
sem quebrar comportamentos, contratos ou dados. A entrega deve ser compreensível
e verificável sobre sua base. Divida por resultado técnico ou funcional, sem
usar quantidade de linhas ou de commits como medida de progresso.

Integração e disponibilização ao usuário são decisões diferentes. Uma nova
capacidade pode entrar desativada se esse estado preservar o comportamento
atual, tiver testes e tiver sua ativação registrada como outra entrega. Uma
migração de dados incompatível ou um contrato quebrado não se tornam seguros
apenas por esconder a interface. Se não houver uma separação segura, reduza o
escopo ou mantenha a entrega bloqueada até reunir a menor unidade segura.

Branches com mais de dois dias úteis motivam uma revisão do tamanho da entrega e
dos bloqueios. Isso não é prazo de merge, motivo para pular revisão ou incentivo
a publicar código incompleto. Priorize concluir uma entrega antes de abrir novas
dependências. Use worktrees separados para trabalhos simultâneos e responsáveis
explícitos para coordenar mudanças em branches publicadas.

## Entregas independentes

Parta da `main` remota atualizada, implemente a entrega e abra o PR para `main`.
Os comandos deste guia usam PowerShell e pressupõem um checkout limpo, sem outra
operação Git em andamento. Não descarte alterações locais para executar exemplos.

```powershell
git fetch origin
git switch --no-track -c nick/initial-cycle/branching-strategy origin/main
```

Antes de integrar, confira o diff contra a base real do PR, valide os commits e
execute os checks aplicáveis. Quando a `main` avançar, atualize por rebase,
revise novamente e repita as verificações. A nota de changelog resume o PR inteiro,
usando seu número real e a base atualizada.

## Entregas dependentes e PRs encadeados

Suponha `changelog` como entrega A e `commit-validation` como B:

```text
Revisão:     PR de A -> main          PR de B -> branch de A
Integração:  A -> main                depois B -> main atualizada
```

1. Crie B a partir da ponta revisada de A. Registre a dependência e o SHA de A
   efetivamente incluído em B, chamado abaixo de `parentBoundary`.
2. Abra o PR de B para a branch de A. Seu diff deve mostrar apenas B. Esse destino
   serve para revisão. Não integre B em A.
3. Se A mudar, realinhe B sobre a nova ponta de A, revise o incremento e atualize
   o limite registrado. Não presuma que a branch ainda aponta para o SHA antigo.
4. Integre A na `main` por rebase merge. Preserve o limite antigo: o GitHub cria
   novos SHAs na integração, mesmo preservando as mudanças de A.
5. Reaplique somente os commits de B sobre a `main` atualizada. Não use o novo SHA
   de A na `main` como limite dos commits antigos de B.
6. Mude a base do PR de B para `main`, confira commits e diff, confira o changelog
   e repita os checks. Integre B somente depois dessa nova validação.

O trecho abaixo demonstra a etapa 5. Substitua os valores pelo registro da entrega
e pelo SHA remoto conferido antes da reescrita. Ele pressupõe uma única
dependência direta e que todo o trabalho remoto conhecido está preservado
localmente. Se algum comando falhar, pare e resolva a causa antes de continuar.

```powershell
git fetch origin
git switch nick/initial-cycle/commit-validation
$parentBoundary = 'SHA_DE_A_INCLUIDO_EM_B'
$remoteBefore = git rev-parse origin/nick/initial-cycle/commit-validation
git merge-base --is-ancestor $remoteBefore HEAD
if ($LASTEXITCODE -ne 0) { throw 'Confira o trabalho remoto antes de reescrever.' }
git merge-base --is-ancestor $parentBoundary HEAD
if ($LASTEXITCODE -ne 0) { throw 'O limite registrado nao pertence a esta branch.' }
git log --oneline "$parentBoundary..HEAD"
git rebase --onto origin/main $parentBoundary
if ($LASTEXITCODE -ne 0) { throw 'Resolva ou aborte o rebase antes de continuar.' }
git diff origin/main...HEAD
```

Revise o resultado e execute as verificações locais da entrega e do changelog.
Somente depois de aprovadas, publique usando a mesma ponta remota conferida:

```powershell
git push --force-with-lease="refs/heads/nick/initial-cycle/commit-validation:$remoteBefore" origin HEAD:refs/heads/nick/initial-cycle/commit-validation
```

Combine a reescrita com quem trabalha na branch. O lease explícito impede
substituir uma ponta remota que mudou desde a conferência. Se ele recusar o push,
revise as mudanças remotas e coordene a retomada. Não substitua por `--force`.

Para atualizar B quando A muda antes de integrar, use o mesmo limite antigo em
`git rebase --onto NOVA_BASE LIMITE_ANTIGO`, com B selecionada. Para cadeias
maiores, atualize da base para a ponta. Uma entrega com várias dependências
independentes aguarda a integração delas na `main`, evitando juntar branches
laterais numa branch agregadora.

Após qualquer mudança de base, confira também a base escolhida na interface do
PR. Trocar a base pode tornar comentários de revisão desatualizados. Exclua A
somente depois de realinhar suas dependentes e redirecionar seus PRs.

## Changelog, conflitos e recuperação

Depois de atualizar a base, execute a verificação da nota usando o número real do
PR, sua base atual e o mesmo contexto versionado usado na geração:

```powershell
clean-dev-cycle changelog --base origin/main --pr 42 --check
```

`42` é apenas um exemplo. Se houver contexto, acrescente
`--context-file .changelog-context/42.md` tanto na geração quanto na verificação.
Se o diff ou o contexto mudar, gere e revise novamente a nota antes de integrar.
Uma mudança apenas de SHA não exige nova nota se o fingerprint continuar válido.

Ao resolver conflitos em `CHANGELOG.md`, preserve entradas e metadados de outros
PRs. Cada entrega descreve somente seu incremento, sem repetir a nota de A em B.
Não altere fingerprints manualmente nem aumente o limite de diff para contornar
uma entrega grande. O limite atual é 128 KiB e exclui o próprio changelog.

Em conflito de rebase, revise os arquivos, resolva preservando a intenção das duas
mudanças, selecione somente os arquivos resolvidos e use `git rebase --continue`.
Se a resolução não estiver clara, `git rebase --abort` retorna ao estado anterior
ao rebase. Registre o bloqueio e peça esclarecimento sobre a intenção conflitante.
Repita os checks após a resolução, mesmo que já tenham passado antes.

Ao cancelar uma entrega, registre o motivo e revise suas dependentes antes de
encerrar PRs ou excluir branches. Uma dependente pode ser cancelada, ficar
bloqueada ou ser adaptada em uma nova revisão para não precisar da predecessora.
Somente a adaptação comprovada remove a dependência.

Se uma entrega integrada causar regressão, priorize uma correção pequena ou um
revert por PR. Preserve o histórico da `main` e registre a recuperação e seu
impacto nas dependentes. Não reescreva a `main` para apagar a entrega.

## Proteções e crescimento

A política alvo exige PR, histórico linear, base atualizada, proteção também para
administradores e os checks `Qualidade` e `Entrega do PR`. Permitir somente
rebase merge e bloquear force-push e exclusão da `main`. `Commits do push` não
substitui o check de entrega do PR.

Ative exigências somente quando seus validadores estiverem integrados e
comprovados. Workflows no repositório não ativam proteções remotas por si só.
Não remova branches automaticamente enquanto houver PRs dependentes delas.

Para crescer, mantenha os planos por escopo, as entregas pequenas e o registro
explícito das dependências. Aumentar o número de pessoas não exige uma branch
permanente por equipe ou escopo. A convenção organiza o trabalho, mas não
substitui revisão, testes, compatibilidade e coordenação.

Git Flow, com `develop` e fluxos de estabilização, não é o modelo desta etapa.
Mantemos uma linha atual, com releases por versão. Manutenção simultânea de
versões antigas, automação de branches e fila de integração exigem entregas
próprias. Antes de habilitar uma fila, adaptar e validar os checks para seus
eventos, incluindo `merge_group`, e a política de merge compatível.

## Referências

- [Trunk-based com branches curtas](https://trunkbaseddevelopment.com/short-lived-feature-branches/): integração frequente e vários PRs ligados ao mesmo objetivo.
- [Git Flow e a reflexão do autor](https://nvie.com/posts/a-successful-git-branching-model/): escolha do fluxo conforme entrega e manutenção de versões.
- [Métodos de merge do GitHub](https://docs.github.com/en/pull-requests/reference/pull-request-merges): preservação dos commits e alteração de SHAs por rebase merge.
- [Rebase e transplante de uma branch](https://git-scm.com/docs/git-rebase): uso de `--onto` e recuperação.
- [Mudança da base de um PR](https://docs.github.com/en/pull-requests/how-tos/create-pull-requests/changing-the-base-branch-of-a-pull-request): revisão após mudar o destino.
