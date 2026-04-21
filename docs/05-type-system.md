# Type System

Verun is statically typed. Every field, parameter, expression, and function return has a known type at compile time. The type checker runs before verification and rejects ill-typed specs before they reach the solver.

## Primitive Types

### `int`

Arbitrary-precision integers in the specification. The SMT solver treats them as mathematical integers (unbounded). In generated code:

| Target     | Maps to    |
|------------|------------|
| Rust       | `i64`      |
| TypeScript | `number`   |
| Solidity   | `int256`   |

```verun
balance: int
count: int
```

### `real`

Real-valued numbers (decimal). Used for specifications that involve continuous quantities:

| Target     | Maps to    |
|------------|------------|
| Rust       | `f64`      |
| TypeScript | `number`   |
| Solidity   | `int256`   |

```verun
rate: real
temperature: real
```

Note: Solidity has no native floating-point type. `real` maps to `int256`, which means you should use fixed-point arithmetic patterns if targeting Solidity with real-valued specs.

### `bool`

Boolean values — `true` or `false`:

| Target     | Maps to |
|------------|---------|
| Rust       | `bool`  |
| TypeScript | `boolean`|
| Solidity   | `bool`  |

```verun
active: bool
frozen: bool
```

### `string`

Text values:

| Target     | Maps to  |
|------------|----------|
| Rust       | `String` |
| TypeScript | `string` |
| Solidity   | `string` |

```verun
name: string
label: string
```

Strings have limited support in SMT verification — the solver cannot reason deeply about string operations. Use strings for metadata fields that don't appear in invariants.

## Collection Types

### Bounded Arrays — `T[N]`

Fixed-size arrays where `T` is the element type and `N` is the compile-time size:

```verun
values: int[8]
flags: bool[16]
scores: real[100]
```

| Target     | Maps to                    |
|------------|----------------------------|
| Rust       | `[i64; 8]`                |
| TypeScript | `number[]`                |
| Solidity   | `int256[8]`               |

Arrays are accessed by integer index:

```verun
values[0]       // first element
values[i]       // element at index i
values[top - 1] // last valid element
```

Array elements can be written in transitions:

```verun
values[i] = 42
values[i] += delta
```

Arrays are the primary tool for reasoning about ordered collections. Combined with `forall` quantifiers, you can express rich properties:

```verun
invariant sorted {
    forall i in 0..length - 1: values[i] <= values[i + 1]
}

invariant all_positive {
    forall i in 0..length: values[i] > 0
}
```

**SMT encoding**: Arrays are encoded using Z3's Array theory (`(Array Int T)`). Read access uses `select`, write access uses `store`. This gives the solver efficient reasoning about arrays without expanding every element.

Arrays do not require initialization in the `init` block. Uninitialized arrays have symbolic (unknown) values in verification — the solver will try all possible contents.

### Maps — `map[K, V]`

Key-value mappings where `K` is the key type and `V` is the value type:

```verun
balances: map[int, int]
permissions: map[int, bool]
```

| Target     | Maps to                              |
|------------|--------------------------------------|
| Rust       | `HashMap<i64, i64>`                 |
| TypeScript | `Map<number, number>`               |
| Solidity   | `mapping(int256 => int256)`         |

Maps are accessed by key:

```verun
balances[account_id]
permissions[user_id]
```

Map values can be written:

```verun
balances[account_id] = new_balance
balances[account_id] += deposit
```

**SMT encoding**: Maps use the same Z3 Array theory as arrays (`(Array K V)`). The solver handles them as total functions from keys to values.

Maps do not require initialization in `init`.

## User-Defined Types

### Enums

Finite sets of named values:

```verun
enum Direction {
    North,
    South,
    East,
    West
}
```

Enums are **value types**. They can be compared with `==` and `!=` and used in field types, parameters, preconditions, and invariants.

```verun
state Navigation {
    heading: Direction

    init {
        heading = Direction::North
    }

    transition turn_right() {
        if heading == Direction::North {
            heading = Direction::East
        } else {
            if heading == Direction::East {
                heading = Direction::South
            } else {
                if heading == Direction::South {
                    heading = Direction::West
                } else {
                    heading = Direction::North
                }
            }
        }
    }
}
```

**SMT encoding**: Enum variants are encoded as distinct integer constants. The solver knows they're all different and that the enum field can only hold one of the declared variants.

### Struct Types (`type`)

Composite types with named fields:

```verun
type Coordinate {
    x: int,
    y: int
}

type Bounds {
    min: int,
    max: int
}
```

Struct fields are accessed with dot notation:

```verun
point.x
bounds.max
```

Struct types are used as field types in state machines:

```verun
state Canvas {
    origin: Coordinate
    limits: Bounds
}
```

### Refinement Types

Refinement types attach a constraint to a base type:

```verun
type PositiveInt = int where value > 0
type Percentage = int where value >= 0 && value <= 100
type NonNegative = int where value >= 0
```

The keyword `value` inside the `where` clause refers to the value being constrained. The constraint must be a boolean expression.

Refinement types are resolved to their base type for storage and code generation — a `PositiveInt` is stored as `int`. The constraint is used during formal verification:

- **As assumption on pre-state:** if a field has type `PositiveInt`, the verifier assumes `value > 0` when checking that transitions preserve invariants.
- **As assumption on parameters:** if a transition parameter has a refined type, the constraint is assumed during verification.
- **Init check:** the init values must satisfy the invariants (including those implied by refinement types).

This makes refinement types a powerful tool for expressing domain constraints without repeating preconditions:

```verun
type PositiveInt = int where value > 0

state Balance {
    amount: PositiveInt

    invariant positive {
        amount > 0
    }

    init {
        amount = 1
    }

    // No precondition needed on delta — the type guarantees it's positive
    transition add(delta: PositiveInt) {
        amount = amount + delta
    }
}
```

## Type Checking Rules

The type checker enforces these rules before verification:

### Assignment Compatibility

The right-hand side of an assignment must match the field's declared type:

```verun
balance: int

// OK
balance = 42
balance += 10

// ERROR: type mismatch — expected int, found bool
balance = true
```

### Compound Assignment Types

Compound operators (`+=`, `-=`, `*=`, `/=`) require both sides to be numeric (`int` or `real`):

```verun
count += 1       // OK: int += int
rate -= 0.5      // OK: real -= real
active += true   // ERROR: bool is not numeric
```

### Comparison and Logical Operators

- Comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`) require operands of the same type.
- Logical operators (`&&`, `||`, `!`) require boolean operands.
- Arithmetic operators require numeric operands.

### Invariant Conditions

Invariant expressions must evaluate to `bool`:

```verun
// OK
invariant valid { balance >= 0 }

// ERROR: invariant must be boolean, found int
invariant broken { balance + 1 }
```

### Indexed Access Types

- Array index must be `int`.
- Array element type matches the declared element type.
- Map key type must match the declared key type.
- Map access returns the declared value type.

```verun
values: int[8]
balances: map[int, int]

values[0]          // type: int
balances[42]       // type: int

values[true]       // ERROR: array index must be int
```

### Indexed Assignment Types

The value being assigned must match the collection's element/value type:

```verun
values: int[8]

values[i] = 42      // OK: int into int[]
values[i] = true    // ERROR: expected int, found bool
```

### `old()` Context

`old()` can only appear inside `ensure` blocks. Using it elsewhere is a type error:

```verun
// OK
ensure { balance == old(balance) - amount }

// ERROR: old() used outside postcondition
where { old(balance) > 0 }
```

### Field Initialization

Every non-collection field must be assigned in `init`. Missing assignments produce a warning:

```verun
state Example {
    x: int
    y: int
    data: int[10]   // OK to skip — collection type

    init {
        x = 0
        // WARNING: field 'y' not initialized
    }
}
```

### Duplicate Detection

The type checker catches duplicate definitions at every level:

- Duplicate enum names
- Duplicate enum variants within an enum
- Duplicate type names
- Duplicate field names within a state
- Duplicate transition names within a state
