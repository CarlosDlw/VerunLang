# Language Guide

This chapter covers the full syntax of Verun — every construct, every operator, every expression form.

## Program Structure

A Verun file is a sequence of top-level items:

```verun
// imports
import "other_module.verun" as other

// enum definitions
enum Status { Active, Inactive }

// type definitions (structs)
type Config {
    max_size: int,
    enabled: bool
}

// function declarations
fn compute(a: int, b: int) -> int

// top-level constants
const LIMIT: int = 100

// state machines
state MySystem {
    // ...
}
```

Items can appear in any order. Enums and types defined at the top level are available to all state machines in the file.

## Comments

```verun
// Single-line comment

/* Multi-line
   block comment */
```

Comments are ignored by the parser and do not appear in the AST.

## Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores:

```
balance
_internal
max_value_2
totalSupply
```

Reserved words cannot be used as identifiers: `state`, `enum`, `type`, `const`, `invariant`, `transition`, `init`, `where`, `ensure`, `emit`, `if`, `else`, `match`, `let`, `forall`, `exists`, `in`, `old`, `true`, `false`, `fn`, `extern`, `import`, `as`, `assert`.

## Literals

### Integer Literals

```verun
0
42
1000000
```

Integers are arbitrary-precision in the specification. In generated code, they map to the target's integer type (`i64` in Rust, `number` in TypeScript, `int256` in Solidity).

### Real Literals

```verun
3.14
0.001
100.0
```

Real numbers use decimal notation with a mandatory decimal point. They map to `f64` in Rust, `number` in TypeScript, and `int256` in Solidity (where true floating-point is unavailable).

### Boolean Literals

```verun
true
false
```

### String Literals

```verun
"hello world"
"escaped \"quotes\""
```

Strings are double-quoted with backslash escaping.

## Operators

### Arithmetic

| Operator | Meaning        | Example       |
|----------|----------------|---------------|
| `+`      | Addition       | `a + b`       |
| `-`      | Subtraction    | `a - b`       |
| `*`      | Multiplication | `a * b`       |
| `/`      | Division       | `a / b`       |
| `%`      | Modulo         | `a % b`       |
| `-`      | Negation       | `-x`          |

### Comparison

| Operator | Meaning                | Example      |
|----------|------------------------|--------------|
| `==`     | Equal                  | `a == b`     |
| `!=`     | Not equal              | `a != b`     |
| `<`      | Less than              | `a < b`      |
| `>`      | Greater than           | `a > b`      |
| `<=`     | Less than or equal     | `a <= b`     |
| `>=`     | Greater than or equal  | `a >= b`     |

### Logical

| Operator | Meaning     | Example         |
|----------|-------------|-----------------|
| `&&`     | Logical AND | `a > 0 && b > 0`|
| `\|\|`   | Logical OR  | `a == 0 \|\| b == 0`|
| `!`      | Logical NOT | `!frozen`       |

### Implication

| Operator | Meaning     | Example         |
|----------|-------------|-----------------|
| `==>`    | Logical implication | `active ==> x > 0` |

The implication operator `a ==> b` is equivalent to `!a || b`. It reads as "if `a` is true, then `b` must also be true". This is particularly useful in invariants:

```verun
invariant frozen_means_zero {
    frozen ==> balance == 0
}
```

This says: whenever the account is frozen, the balance must be zero. If the account is not frozen, the invariant is trivially satisfied.

`==>` has the lowest precedence among logical operators (below `||`), so `a && b ==> c || d` is parsed as `(a && b) ==> (c || d)`.

### Ranges

```verun
0..length
```

Ranges create a sequence from start (inclusive) to end (exclusive). They are primarily used in `forall` and `exists` expressions as iteration domains.

## Expressions

### Field Access

Access fields of custom types using dot notation:

```verun
config.max_size
account.owner.name
```

### Array Access

Access array elements by integer index:

```verun
values[0]
data[i]
buffer[top - 1]
```

### Map Access

Access map values by key:

```verun
balances[account_id]
scores[player_id]
```

Syntactically identical to array access — the type system determines whether it's an array index or a map lookup.

### Function Calls

```verun
compute(a, b)
hash(data, length)
```

Functions must be declared before use (see [Functions](#functions) below), except for builtin functions.

### Builtin Functions

Verun provides a small set of builtin functions available without declaration:

| Function | Arguments | Return Type | Description |
|----------|-----------|-------------|-------------|
| `abs(x)` | 1 numeric | Same as input | Absolute value |
| `min(a, b)` | 2 numeric | Same as inputs | Minimum of two values |
| `max(a, b)` | 2 numeric | Same as inputs | Maximum of two values |

Builtins work in all contexts — invariants, preconditions, postconditions, init blocks, and transition bodies:

```verun
state Clamped {
    x: int
    lo: int
    hi: int

    invariant bounded {
        x >= lo && x <= hi
    }

    init {
        lo = 0
        hi = 100
        x = 50
    }

    transition set(val: int) {
        x = min(max(val, lo), hi)
    }
}
```

In the SMT solver, builtins are encoded as ITE (if-then-else) expressions:
- `abs(x)` → `if x >= 0 then x else -x`
- `min(a, b)` → `if a <= b then a else b`
- `max(a, b)` → `if a >= b then a else b`

In generated code, they map to native equivalents: `.abs()` / `.min()` / `.max()` in Rust, `Math.abs()` / `Math.min()` / `Math.max()` in TypeScript, and ternary expressions in Solidity.

### Enum Variants

```verun
Phase::Active
Status::Closed
Color::Red
```

Double-colon syntax, matching the enum definition.

### `old()` Expressions

Only valid inside `ensure` blocks:

```verun
ensure {
    balance == old(balance) - amount
}
```

Captures the pre-transition value of any field expression.

### Quantifiers

#### `forall`

Universal quantification — "for every element in the domain, the condition holds":

```verun
forall i in 0..length: values[i] >= 0
```

This says: for every index `i` from 0 to `length - 1`, the value at that index is non-negative.

In the SMT solver, bounded `forall` over ranges is expanded or encoded as array theory constraints. This is the primary way to express properties over collections.

#### `exists`

Existential quantification — "there is at least one element in the domain where the condition holds":

```verun
exists i in 0..length: values[i] == target
```

This says: there is some index where the value equals `target`.

### Parenthesized Expressions

```verun
(a + b) * c
!(x > 0 && y > 0)
```

Use parentheses for explicit grouping when operator precedence isn't obvious.

## Statements

Statements appear in transition bodies and modify state.

### Simple Assignment

```verun
balance = 0
phase = Phase::Active
name = "default"
```

### Compound Assignment

```verun
balance += amount     // balance = balance + amount
balance -= amount     // balance = balance - amount
counter *= 2          // counter = counter * 2
total /= parts       // total = total / parts
```

### Indexed Assignment

Write to a specific position in an array or map:

```verun
values[i] = 42
data[top] = new_value
balances[account_id] = 0
```

### Indexed Compound Assignment

```verun
values[i] += delta
scores[player_id] -= penalty
```

### Let Bindings

Use `let` to create transition-local variables:

```verun
let amount_with_fee: int = amount + fee
let normalized = max(amount_with_fee, 0)
```

`let` bindings are scoped to the transition execution and can be used by later statements in the same transition.

### Conditional Statements

```verun
if balance > threshold {
    status = Phase::Active
} else {
    status = Phase::Inactive
}
```

The `else` branch is optional. Nesting is supported:

```verun
if x > 100 {
    x = 100
} else {
    if x < 0 {
        x = 0
    } else {
        x = x + delta
    }
}
```

    `else if` chains are also supported directly:

    ```verun
    if score < 0 {
        status = Status::Rejected
    } else if score < 70 {
        status = Status::Review
    } else {
        status = Status::Approved
    }
    ```

    ### Match Statements

    Pattern matching supports enum variants, primitive literals, and wildcard `_`:

    ```verun
    match phase {
        Phase::Draft => {
            score = 0
        },
        Phase::Review => {
            score = input
        },
        _ => {
            score = max(input, 1)
        }
    }
    ```

    For enum matches, type checking enforces exhaustiveness unless a wildcard arm is present.

### Assert

```verun
assert index >= 0
assert amount <= max_allowed
```

Adds a runtime check in generated code and a constraint in the SMT model.

## Enum Definitions

Enums define a finite set of named values:

```verun
enum OrderStatus {
    Pending,
    Confirmed,
    Shipped,
    Delivered,
    Cancelled
}
```

Enums are value types. They can be used as field types, compared with `==` and `!=`, and assigned in transitions and init blocks.

```verun
state Order {
    status: OrderStatus

    init {
        status = OrderStatus::Pending
    }

    transition confirm() {
        where { status == OrderStatus::Pending }
        status = OrderStatus::Confirmed
    }
}
```

Trailing commas are allowed in variant lists.

## Type Definitions

### Struct Types

Custom composite types (structs):

```verun
type Coordinate {
    x: int,
    y: int
}

type UserProfile {
    id: int,
    active: bool
}
```

Type fields are accessed with dot notation in expressions. Fields are comma-separated with an optional trailing comma.

### Refinement Types

Refinement types define a base type with an attached constraint. The syntax is:

```verun
type Name = base_type where constraint
```

The `value` keyword inside the constraint refers to the value being constrained:

```verun
type PositiveInt = int where value > 0
type Percentage = int where value >= 0 && value <= 100
type NonNegative = int where value >= 0
```

Refinement types can be used as field types and parameter types:

```verun
type PositiveInt = int where value > 0

state Account {
    balance: PositiveInt

    invariant positive {
        balance > 0
    }

    init {
        balance = 100
    }

    transition deposit(amount: PositiveInt) {
        balance = balance + amount
    }
}
```

**How refinement types work in verification:**

When a field or parameter has a refined type, the SMT solver automatically injects the refinement constraint as an assumption. In the example above:
- The solver assumes `balance > 0` on the pre-state (from the `PositiveInt` refinement)
- The solver assumes `amount > 0` for the `deposit` parameter
- These assumptions help prove invariants that depend on the values being positive

This means the `deposit` transition above will pass verification — the solver knows both `balance` and `amount` are positive, so `balance + amount` is also positive.

Refinement types are resolved to their underlying type for SMT encoding and code generation. A `PositiveInt` field is stored as a regular `int` — the constraint is enforced at the specification level, not at runtime.

**Type checking:** The refinement expression must evaluate to `bool`. The `value` variable is typed as the base type. If the refinement is not boolean, you get a type error.

## Functions

Functions declare operations that can be used in expressions. They come in two forms:

### Extern Functions

Declare a function signature without a body. In verification, the SMT solver treats it as an **uninterpreted function** — it knows nothing about what it computes, only its type signature:

```verun
fn hash(data: int, nonce: int) -> int
```

This is useful for abstracting over computations that the verifier doesn't need to reason about. The solver will only use the fact that `hash(a, b) == hash(a, b)` for the same inputs (referential transparency).

### Extern-Qualified Functions

```verun
extern fn oracle_price(asset_id: int) -> int
```

Explicitly marks the function as externally defined. Semantically equivalent to a bodyless `fn` in the current version.

### Using Functions in Expressions

```verun
invariant bounded {
    compute(balance, fee) >= 0
}

transition process(x: int) {
    where { hash(x, nonce) != 0 }
    result = compute(x, factor)
}
```

## Import Declarations

```verun
import "utils.verun"
import "types/common.verun" as common
```

Import declarations are resolved recursively at compile/check time using relative paths from the current file. Import cycles are detected and reported as errors. Imported items are merged into the compilation unit.

`as` aliases are supported for namespace-qualified symbol references.

Use `alias::Name` for imported types/constants/functions and `alias::Enum::Variant` for enum variants:

```verun
import "types/common.verun" as common

state Workflow {
    phase: common::Phase

    init { phase = common::Phase::Draft }

    transition approve() {
        where { common::MAX_RETRIES >= 0 }
        phase = common::Phase::Approved
        ensure { phase == common::Phase::Approved }
    }
}
```

If `alias::Name` refers to a symbol that does not exist in the imported file, import resolution fails.

Function calls through alias namespaces are also supported, including inside `let`, `where`, and `match`-driven transitions:

```verun
import "types/common.verun" as common

state Pipeline {
    mode: common::Mode
    value: int

    init {
        mode = common::Mode::A
        value = 0
    }

    transition step(delta: int) {
        let normalized: int = common::normalize(delta)
        value = normalized

        let next: common::Mode = common::Mode::B
        match next {
            common::Mode::A => {
                mode = common::Mode::A
            }
            common::Mode::B => {
                mode = common::Mode::B
            }
        }
    }
}
```

For function calls (aliased or not), type checking validates argument count and argument types against the declared function signature.

## Constant Declarations

Constants can be declared at top-level or inside a `state` block:

```verun
const MAX_USERS: int = 1000

state Registry {
    const MIN_AGE: int = 18
    count: int
    init { count = 0 }
}
```

Constants are immutable and can be referenced in invariants, preconditions, postconditions, and transition bodies.

## Semicolons and Separators

Verun does **not** use semicolons. Statements are newline-separated:

```verun
transition update(n: int) {
    where { n > 0 }
    value += n
    count += 1
}
```

Commas separate items in:
- Function arguments: `compute(a, b, c)`
- Enum variants: `Active, Inactive, Closed`
- Type/struct fields: `x: int, y: int`

Newlines separate items in:
- Init assignments: `value = 0` then `max = 100` on the next line
- Where conditions: `amount > 0` then `balance >= amount` on the next line
- Ensure conditions: `balance == old(balance) - amount` then `reserve == old(reserve) + amount` on the next line

Trailing commas are allowed in enum variant lists and struct field lists.

## Whitespace and Formatting

Whitespace (spaces, tabs, newlines) is insignificant except as a token separator. You can write compact or expanded:

```verun
// Compact
invariant valid { x >= 0 && x <= 100 }

// Expanded
invariant valid {
    x >= 0 && x <= 100
}
```

Use `verun fmt` to normalize formatting to the canonical style.
