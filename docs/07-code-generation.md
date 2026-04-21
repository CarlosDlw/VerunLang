# Code Generation

Verun generates implementation code from verified specs. The generated code carries the guarantees encoded in your spec — precondition checks, postcondition assertions, and invariant enforcement — all derived from the formal specification.

**Code generation requires successful verification.** If `verun check` reports failures, `verun gen` will refuse to generate code. This ensures you never ship unverified implementations.

## Supported Targets

| Target       | Flag            | Extension | Description                        |
|-------------|-----------------|-----------|-------------------------------------|
| Rust         | `-t rust`       | `.rs`     | Structs with methods               |
| TypeScript   | `-t typescript` | `.ts`     | Classes with methods               |
| Solidity     | `-t solidity`   | `.sol`    | Smart contracts                    |
| Java         | `-t java`       | `.java`   | Classes with methods               |
| Go           | `-t go`         | `.go`     | Structs with methods               |
| C            | `-t c`          | `.c`      | Structs + transition functions     |
| Move         | `-t move`       | `.move`   | Module-oriented on-chain code      |
| Cairo        | `-t cairo`      | `.cairo`  | Starknet-oriented contract code    |
| Vyper        | `-t vyper`      | `.vy`     | Python-like smart contract code    |

Short aliases work too: `-t rs`, `-t ts`, `-t sol`, `-t vy`.

## How Specs Map to Code

The mapping from Verun constructs to generated code follows consistent patterns across all targets.

### States → Structs / Classes / Contracts

A Verun `state` becomes the primary data structure in the target language:

| Verun          | Rust             | TypeScript       | Solidity            |
|----------------|------------------|------------------|---------------------|
| `state Token`  | `pub struct Token` | `export class Token` | `contract Token`  |
| fields         | `pub` fields      | public properties | `public` state vars |

### Init → Constructor

The `init` block becomes the constructor or factory method:

| Target     | Pattern                                    |
|------------|---------------------------------------------|
| Rust       | `pub fn new() -> Self { Self { ... } }`    |
| TypeScript | `constructor() { this.field = value; }`    |
| Solidity   | `constructor() { field = value; }`         |

### Transitions → Methods / Functions

Each transition becomes a callable method:

| Target     | Signature Pattern                                              |
|------------|----------------------------------------------------------------|
| Rust       | `pub fn transition_name(&mut self, params) -> Result<(), String>` |
| TypeScript | `public transition_name(params): void`                        |
| Solidity   | `function transition_name(params) external checkInvariants`   |

### Preconditions → Guards

`where` clauses become input validation at the top of each method:

| Target     | Pattern                                              |
|------------|------------------------------------------------------|
| Rust       | `if !(condition) { return Err("precondition failed".to_string()); }` |
| TypeScript | `if (!(condition)) { throw new Error("precondition failed"); }` |
| Solidity   | `require(condition, "precondition failed");`         |

### Postconditions → Assertions

`ensure` clauses become assertions after the body executes. The `old()` values are captured before the body runs:

**Rust:**
```rust
let _old_balance = self.balance.clone();
// ... body ...
debug_assert!(self.balance == (_old_balance - amount), "postcondition violated in 'withdraw'");
```

**TypeScript:**
```typescript
const _old_balance = this.balance;
// ... body ...
if (!(this.balance === (_old_balance - amount))) {
    throw new Error("postcondition violated in 'withdraw'");
}
```

**Solidity:**
```solidity
int256 _old_balance = balance;
// ... body ...
require(balance == (_old_balance - amount), "postcondition violated");
```

### Invariants → Runtime Checks

Invariants are checked after every transition to catch violations at runtime:

**Rust:** `debug_assert!` after each method body — active in debug builds, optimized away in release.

**TypeScript:** `if (!condition) throw new Error(...)` at the end of each method.

**Solidity:** A `modifier checkInvariants()` is applied to every function. The modifier runs the body first, then checks all invariant conditions with `require()`:

```solidity
modifier checkInvariants() {
    _;
    require(balance >= 0, "invariant 'non_negative' violated");
    require(balance <= max_balance, "invariant 'bounded' violated");
}
```

### Enums

| Target     | Pattern                                                    |
|------------|-------------------------------------------------------------|
| Rust       | `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum`   |
| TypeScript | `export enum Phase { Active = "Active", ... }`             |
| Solidity   | `enum Phase { Active, Paused, Closed }`                    |

### Arrays and Maps

| Verun Type      | Rust                      | TypeScript       | Solidity                    |
|-----------------|---------------------------|------------------|-----------------------------|
| `int[8]`        | `[i64; 8]`               | `number[]`       | `int256[8]`                |
| `map[int, int]` | `HashMap<i64, i64>`      | `Map<number, number>` | `mapping(int256 => int256)` |

### Events

`emit` statements generate target-appropriate event code:

| Target     | Pattern                                     |
|------------|----------------------------------------------|
| Rust       | (returned as event data — target handles dispatch) |
| TypeScript | (not yet emitted — placeholder for event bus) |
| Solidity   | `event Transfer(uint256 arg0); ... emit Transfer(amount);` |

Solidity events are automatically declared at the contract level based on `emit` usage in transitions.

### Assertions

`assert` statements in transition bodies:

| Target     | Pattern                                              |
|------------|------------------------------------------------------|
| Rust       | `assert!(condition);`                                |
| TypeScript | `if (!(condition)) { throw new Error("assertion failed"); }` |
| Solidity   | `require(condition, "assertion failed");`            |

### Implication Operator (`==>`)

The `==>` operator is compiled to boolean logic across all targets:

| Target     | `a ==> b` compiles to |
|------------|------------------------|
| Rust       | `(!a \|\| b)`          |
| TypeScript | `(!a \|\| b)`          |
| Solidity   | `(!a \|\| b)`          |

### Builtin Functions

| Function | Rust | TypeScript | Solidity |
|----------|------|-----------|----------|
| `abs(x)` | `x.abs()` | `Math.abs(x)` | `x >= 0 ? x : -x` |
| `min(a, b)` | `a.min(b)` | `Math.min(a, b)` | `a <= b ? a : b` |
| `max(a, b)` | `a.max(b)` | `Math.max(a, b)` | `a >= b ? a : b` |

### Refinement Types

Refinement types are resolved to their base type during code generation. A field of type `PositiveInt` (defined as `type PositiveInt = int where value > 0`) generates as a regular `int` / `i64` / `int256` field. The refinement constraint is enforced at the specification level through formal verification, not at runtime.

## Rust Target Details

The Rust target generates idiomatic Rust with:

- `#[derive(Debug, Clone)]` on structs
- `pub` fields for external access
- `Result<(), String>` return types for transitions (precondition failures return `Err`)
- `debug_assert!` for invariants and postconditions
- `clone()` for capturing `old()` values

The generated code compiles standalone — no Verun runtime dependency. Add it to your project as a module.

## TypeScript Target Details

The TypeScript target generates:

- Exported classes and enums
- Typed properties and method signatures
- Exception-based precondition and invariant enforcement
- `const` bindings for `old()` captures

The output is valid TypeScript that works in Node.js or browser environments with no dependencies.

## Solidity Target Details

The Solidity target generates production-ready smart contract code:

- `pragma solidity ^0.8.20;` and SPDX license header
- State variables with `public` visibility (auto-generates getters)
- `external` functions for gas efficiency
- `require()` for all runtime checks (preconditions, postconditions, invariants, assertions)
- `modifier checkInvariants()` pattern — checks run after every state-modifying function
- Proper data locations: value types (int, bool, enum) without `memory`, reference types (arrays, strings) with `memory` for local copies
- `event` declarations auto-generated from `emit` usage

The generated Solidity is designed to be deployed directly or used as a verified base that you extend with additional functionality (access control, external calls, etc.).

### Type Mappings for Solidity

| Verun       | Solidity                         |
|-------------|-----------------------------------|
| `int`       | `int256`                         |
| `real`      | `int256` (no native float)       |
| `bool`      | `bool`                           |
| `string`    | `string`                         |
| `int[N]`    | `int256[N]`                      |
| `map[K, V]` | `mapping(K => V)`               |
| enum        | `enum` (global scope)            |

## Using Generated Code

### Workflow

```
spec.verun → verun check → verun gen -t rust → MyState.rs → integrate into project
```

1. Write the spec.
2. Verify it passes all checks.
3. Generate code for your target.
4. Integrate the generated file into your project.
5. Call the generated methods from your application logic.

### Regeneration

If you change the spec, re-verify and regenerate:

```bash
verun check spec.verun && verun gen spec.verun -t rust -o src/state.rs
```

The generated code is **not meant to be manually edited**. Treat it as a build artifact. If you need to change behavior, change the spec and regenerate.

### Extending Generated Code

For Rust and TypeScript, you can wrap the generated struct/class in your own module that adds non-verified behavior (I/O, logging, persistence):

```rust
mod generated {
    include!("state.rs");
}

impl MyApp {
    fn handle_deposit(&mut self, amount: i64) -> Result<(), String> {
        self.state.deposit(amount)?;
        self.db.save(&self.state);
        Ok(())
    }
}
```

For Solidity, you can inherit from the generated contract or compose it:

```solidity
import "./Token.sol";

contract MyToken is Token {
    // Additional logic
}
```
