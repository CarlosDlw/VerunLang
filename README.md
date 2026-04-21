# Verun

**Programming by Executable Invariants** — a formal specification language where systems are modeled as state machines verified by an SMT solver.

You declare *what must be true*, not *how to verify it*. Verun mathematically proves that no transition ever violates a declared property, then generates code for multiple targets.

---

## How it works

A Verun program defines a **state** with:

- **Typed fields** — the state of the machine
- **Invariants** — properties that must hold in every reachable state
- **Init** — initial state (verified against invariants)
- **Transitions** — the only operations that mutate state, with preconditions (`where`), postconditions (`ensure`), and side effects (`emit`)

The SMT solver (Z3) verifies inductively: given any state satisfying the invariants plus the precondition, the resulting state also satisfies all invariants.

---

## Example

```verun
// ERC-20-like token with formally verified supply conservation

state Token {
    total_supply: int
    balance_a: int
    balance_b: int

    invariant supply_conservation {
        balance_a + balance_b == total_supply
    }

    invariant non_negative_balances {
        balance_a >= 0 && balance_b >= 0
    }

    init {
        total_supply = 1000
        balance_a = 1000
        balance_b = 0
    }

    transition transfer_a_to_b(amount: int) {
        where {
            amount > 0
            amount <= balance_a
        }
        balance_a -= amount
        balance_b += amount
        ensure {
            balance_a == old(balance_a) - amount
            balance_b == old(balance_b) + amount
            total_supply == old(total_supply)
        }
        emit Transfer(amount)
    }
}
```

---

## Requirements

- Rust `>=` 1.85 (edition 2024)
- Z3 installed and available on `PATH`

```bash
# Debian/Ubuntu
sudo apt install z3

# macOS
brew install z3
```

---

## Installation

```bash
git clone https://github.com/CarlosDlw/VerunLang
cd VerunLang
cargo build --release
# binary at target/release/verun
```

---

## CLI

### `verun check` — formal verification via SMT

```bash
verun check examples/token.verun
verun check examples/auction.verun --verbose
verun check examples/counter.verun --format json
```

Verifies: init satisfies invariants, every transition preserves all invariants, postconditions are satisfied.

---

### `verun run` — execution with the runtime engine

```bash
# Initialize state and display it
verun run examples/counter.verun --show-state

# Execute a transition
verun run examples/counter.verun -t increment(10) -s

# Execute and display emitted events
verun run examples/token.verun -t transfer_a_to_b(200) -s
```

---

### `verun gen` — code generation

```bash
verun gen examples/token.verun --target rust
verun gen examples/auction.verun --target solidity -o Auction.sol
verun gen examples/counter.verun --target typescript
```

**Supported targets:** `rust`, `typescript`, `solidity`, `java`, `go`, `c`, `move`, `cairo`, `vyper`

---

### `verun init` — scaffold a new spec

```bash
verun init MyContract
verun init MyContract -o my_contract.verun
```

---

### `verun fmt` — formatting

```bash
verun fmt examples/counter.verun
verun fmt examples/counter.verun --check   # check only, do not write
```

---

### `verun ast` — AST dump (debug)

```bash
verun ast examples/counter.verun
verun ast examples/counter.verun --format json
```

---

## Language

### Types

| Type | Description |
|------|-------------|
| `int` | Integer |
| `real` | Real number (rational in the solver) |
| `bool` | Boolean |
| `string` | String |
| `enum` | Enumerated type with named variants |
| `T[N]` | Bounded fixed-size array of length N |
| `map[K, V]` | Finite map from K to V |
| `type Name { ... }` | Struct with named fields |
| `type Name = T where P` | Refinement type with a constraint |

### Operators

- **Arithmetic**: `+` `-` `*` `/` `%`
- **Comparison**: `==` `!=` `<` `>` `<=` `>=`
- **Logical**: `&&` `||` `!`
- **Implication**: `==>`
- **Assignment**: `=` `+=` `-=` `*=` `/=`
- **Range**: `..` (in quantifiers)

### Quantifiers

```verun
invariant all_positive {
    forall i in 0..size { values[i] >= 0 }
}
```

### Built-in functions

| Function | Description |
|----------|-------------|
| `abs(x)` | Absolute value |
| `min(a, b)` | Minimum |
| `max(a, b)` | Maximum |

### `old()` in postconditions

Inside `ensure`, `old(expr)` refers to the value of `expr` before the transition executed:

```verun
ensure {
    balance == old(balance) - amount
}
```

---

## Included examples

| File | Description |
|------|-------------|
| `examples/counter.verun` | Counter with verified bounds |
| `examples/token.verun` | ERC-20-like token with supply conservation |
| `examples/auction.verun` | Auction with phase transitions |
| `examples/voting.verun` | Voting system |
| `examples/ledger.verun` | Financial ledger |
| `examples/escrow.verun` | Escrow contract |
| `examples/order.verun` | Order state machine |
| `examples/stack.verun` | Stack with bounds invariant |
| `examples/buffer.verun` | Bounded buffer array |
| `examples/rate_limiter.verun` | Rate limiter |
| `examples/thermostat.verun` | Thermostat |
| `examples/match_workflow.verun` | Match expressions and enums |

---

## Project structure

```
src/
  ast/        — AST nodes, types, spans
  cli/        — command-line interface (clap)
  codegen/    — code generation per target
  errors/     — diagnostics and error rendering
  parser/     — parser (pest) + error formatting
  runtime/    — execution engine
  smt/        — Z3 encoding and interface
  types/      — type checker
tests/
  parser_tests.rs
  type_tests.rs
  smt_tests.rs
  runtime_tests.rs
  codegen_tests.rs
  import_tests.rs
examples/     — example specs
```

---

## Tests

```bash
cargo test
```

---

## License

MIT
