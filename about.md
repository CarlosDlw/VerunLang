# Verun — Programming by Executable Invariants

## Conceito

Linguagem de especificação formal onde sistemas são modelados como **máquinas de estado com invariantes verificados formalmente** por SMT solver. O programador declara *o que deve ser verdade*, não *como verificar*. O compilador prova matematicamente que nenhuma transição viola as propriedades declaradas.

Verun não é uma linguagem de propósito geral. É uma linguagem de **especificação verificável** que pode gerar código para targets reais (Solidity, Java, Rust, Go, TypeScript, C, Move, Cairo, Vyper).

## Modelo de Execução

1. **States** — máquinas de estado com campos tipados
2. **Invariants** — propriedades globais que devem valer em todos os estados alcançáveis
3. **Init** — estado inicial (deve satisfazer todas as invariantes)
4. **Transitions** — únicas operações que mutam estado
   - **Preconditions** (`where`) — condições que devem valer antes da execução
   - **Postconditions** (`ensure`) — condições que devem valer depois, com acesso ao pré-estado via `old()`
   - **Side effects** (`emit`) — ações observáveis (logs, eventos)
5. **Verificação indutiva** — dado um estado que satisfaz as invariantes + precondição, o solver prova que o estado resultante também satisfaz todas as invariantes

## Tipos de Dados

| Tipo | Descrição |
|------|-----------|
| `int` | Inteiro de precisão arbitrária |
| `real` | Número real (racional no solver) |
| `bool` | Booleano |
| `string` | String |
| `enum` | Tipo enumerado com variantes nomeadas |
| `T[N]` | Array bounded de tamanho fixo N |
| `map[K, V]` | Mapa finito de K para V |
| `type Name { ... }` | Struct/record com campos nomeados |
| `type Name = T where P` | Refinement type (tipo refinado com constraint) |

### Tipos planejados

| Tipo | Descrição |
|------|-----------|
| `seq<T>` | Sequência dinâmica bounded |
| Dependent types leves | Refinements que referenciam campos do estado |

## Keywords

| Keyword | Função |
|---------|--------|
| `state` | Declara máquina de estado |
| `enum` | Declara tipo enumerado |
| `type` | Declara struct/record |
| `invariant` | Propriedade global verificada formalmente |
| `transition` | Operação que muta estado |
| `init` | Bloco de inicialização |
| `where` | Precondição de transition |
| `ensure` | Postcondição de transition |
| `old` | Referência ao pré-estado dentro de ensure |
| `emit` | Side effects (log, eventos) |
| `forall` | Quantificador universal |
| `exists` | Quantificador existencial |
| `in` | Domínio de quantificador (range ou collection) |
| `import` | Import de módulo |
| `as` | Alias de import |
| `ffi` | Foreign function interface |
| `fn` | Função (FFI ou pura) |
| `extern` | Declaração externa |
| `true` / `false` | Literais booleanos |
| `map` | Tipo mapa |
| `if` / `else` | Condicional dentro de transitions |
| `assert` | Assertion local (diferente de invariant global) |
| `abs` / `min` / `max` | Funções builtin |

### Keywords planejadas

| Keyword | Função |
|---------|--------|
| `trace` | Sequência nomeada de transitions |
| `reachable` | Bounded model checking |
| `always` / `never` | Propriedades temporais (LTL subset) |
| `leads_to` | Liveness property |

## Operadores

- **Aritméticos**: `+` `-` `*` `/` `%`
- **Comparação**: `==` `!=` `<` `>` `<=` `>=`
- **Lógicos**: `&&` `||` `!`
- **Implicação**: `==>` (logical implication: `a ==> b` equivale a `!a || b`)
- **Atribuição**: `=` `+=` `-=` `*=` `/=`
- **Range**: `..` (usado em quantificadores)

## Funções Builtin

| Função | Descrição | Exemplo |
|--------|-----------|--------|
| `abs(x)` | Valor absoluto | `abs(-5)` → `5` |
| `min(a, b)` | Menor entre dois valores | `min(3, 7)` → `3` |
| `max(a, b)` | Maior entre dois valores | `max(3, 7)` → `7` |

## Verificação Formal

- SMT solver (Z3) prova propriedades automaticamente
- **Init check** — estado inicial satisfaz todas as invariantes
- **Transition check** — cada transition preserva todas as invariantes (verificação indutiva 1-step)
- **Postcondition check** — ensure blocks são satisfeitos
- **Dead transition detection** — precondição impossível de satisfazer (warning)
- **Counterexamples** — quando uma verificação falha, o solver extrai valores concretos que demonstram a violação

### Verificação planejada

- Bounded model checking (exploração de traces até profundidade K)
- Propriedades temporais (always, never, leads_to)
- Detecção de deadlocks
- Counterexample traces visuais

## Code Generation

Spec verificada → código compilável para:

- **Solidity** — smart contracts Ethereum
- **Java** — aplicações corporativas
- **Rust** — sistemas seguros
- **TypeScript** — aplicações web
- **Go** — serviços backend
- **C** — embarcados, safety-critical
- **Move** — Sui/Aptos L1
- **Cairo** — Starknet L2
- **Vyper** — smart contracts alternativos

## Tooling

- **CLI** — `verun check`, `verun run`
- **Runtime** — execução direta de specs com engine de estado

## Filosofia

- Declarativo, não imperativo
- O programador especifica *propriedades*, o solver *prova*
- Erros são encontrados em tempo de compilação, não em runtime
- Specs são a fonte de verdade — código gerado é derivado
- Composição de sistemas via módulos e imports
- Verificação é obrigatória, não opcional
