# Core Concepts

Verun programs model systems as **state machines with formally verified invariants**. This chapter explains the foundational concepts that everything else in the language builds on.

## State Machines

Every Verun spec centers on a `state` block. A state machine has:

1. **Fields** — the data it holds
2. **Invariants** — rules that must always be true
3. **Init** — the starting values
4. **Transitions** — the operations that change the data

```verun
state TrafficLight {
    green_time: int
    cycle_count: int

    invariant positive_timing {
        green_time > 0
    }

    invariant non_negative_cycles {
        cycle_count >= 0
    }

    init {
        green_time = 30
        cycle_count = 0
    }

    transition advance() {
        cycle_count += 1
    }

    transition adjust_timing(new_time: int) {
        where { new_time > 0 }
        green_time = new_time
    }
}
```

The state machine is the fundamental unit of specification. You can have multiple state machines in a single file, and they can reference shared enums and types.

## Fields

Fields are the data that your state machine holds. Each field has a name and a type:

```verun
state Wallet {
    balance: int
    owner: string
    active: bool
    limits: int[5]
    tags: map[int, int]
}
```

Fields are the only mutable data in a spec. Transitions modify fields; invariants constrain them; the init block sets their starting values.

All non-collection fields must be initialized in the `init` block. Arrays and maps can be left uninitialized (they start with default/symbolic values).

## Invariants

An invariant is a property that must hold **in every reachable state** of the machine — after initialization and after every transition.

```verun
invariant supply_conservation {
    balance_a + balance_b == total_supply
}
```

Invariants are named for clarity in verification output. When verification runs, each transition is checked against every invariant independently. If transition `T` can reach a state where invariant `I` is false, you get a `[FAIL]` with a counterexample.

### What "Always True" Means

"Always true" doesn't mean "true right now". It means: **for any reachable state where all invariants hold, if you apply any valid transition (one whose preconditions are satisfied), the resulting state also satisfies all invariants.**

This is an inductive proof:
1. The init state satisfies all invariants (base case).
2. Every transition preserves all invariants (inductive step).
3. Therefore, every reachable state satisfies all invariants.

### Multiple Invariants

A state can have any number of invariants. Each is verified independently:

```verun
state Auction {
    highest_bid: int
    min_bid: int
    bid_count: int

    invariant bid_above_minimum {
        highest_bid >= min_bid
    }

    invariant non_negative_bids {
        bid_count >= 0
    }
}
```

Invariants can reference any state field and use any expression — arithmetic, boolean logic, comparisons, quantifiers over arrays. The only requirement is that the expression must evaluate to a boolean.

### Unnamed Invariants

If you don't need a name, you can omit it:

```verun
invariant {
    balance >= 0
}
```

The verification output will label it as "invariant" in that case.

## Initialization

The `init` block defines the starting state. It must satisfy all invariants — this is verified as the first check:

```verun
init {
    balance = 100
    max_balance = 1000
    frozen = false
}
```

If your init values violate an invariant, verification fails immediately:

```
[FAIL] init satisfies invariant 'balance_bounded'
       Counterexample: balance = 100, max_balance = 50
```

Assignments in `init` are newline-separated and must cover every non-collection field.

## Transitions

Transitions are the operations that modify state. They are the only way state changes — there is no implicit mutation.

A transition can have:
- **Parameters** — input values
- **Preconditions** (`where`) — conditions that must be true before the transition executes
- **Body** — the state mutations
- **Postconditions** (`ensure`) — conditions that must be true after the transition executes
- **Events** (`emit`) — side effects to signal

```verun
transition transfer(from_id: int, to_id: int, amount: int) {
    where {
        amount > 0
        amount <= balance
    }

    balance -= amount
    reserve += amount

    ensure {
        balance == old(balance) - amount
        reserve == old(reserve) + amount
    }

    emit Transfer(amount)
}
```

### Preconditions (`where`)

Preconditions guard when a transition can fire. If a precondition is false, the transition simply doesn't happen — it's not an error, it's an invalid operation for the current state.

```verun
where {
    amount > 0
    amount <= balance
    frozen == false
}
```

In generated code, preconditions become `require()` statements (Solidity), early-return checks (Rust), or thrown exceptions (TypeScript).

During verification, the solver assumes all preconditions are true and then checks whether invariants still hold after the body executes. This means preconditions are your tool for restricting the input space — if you need `amount` to be positive, say so in `where`, and the solver will only consider positive amounts.

### Postconditions (`ensure`)

Postconditions specify what must be true after the transition executes. They serve as a contract: "if you call this transition with valid inputs, here's what you can count on."

```verun
ensure {
    balance == old(balance) - amount
    total_supply == old(total_supply)
}
```

The `old()` function refers to the value of a field **before** the transition body ran. This lets you express relationships between the pre-state and post-state.

Postconditions are verified independently — the solver checks that the body's mutations, combined with the preconditions, logically imply each postcondition.

### Events (`emit`)

Events are side effects that transitions can produce. They don't affect state — they're signals to the outside world:

```verun
emit Transfer(amount)
emit AuctionClosed(highest_bid)
```

In generated code:
- **Solidity**: `emit Transfer(amount);` (Solidity events)
- **Rust**: Returned as event structs
- **TypeScript**: Callback or event emitter calls

Events are not verified by the SMT solver — they're purely informational.

## The `old()` Function

`old(expr)` is only valid inside `ensure` blocks. It captures the value of an expression as it was **before the transition body executed**:

```verun
transition increment(n: int) {
    where { n > 0 }
    value += n
    ensure {
        value == old(value) + n
    }
}
```

After `value += n`, `value` is the new value and `old(value)` is what it was before. The postcondition says "the new value equals the old value plus n" — a precise functional specification of what `increment` does.

You can use `old()` on any field:

```verun
ensure {
    balance == old(balance) - amount,
    total_supply == old(total_supply)    // unchanged
}
```

The second postcondition explicitly states that `total_supply` should not change during this transition. This is important — without it, the solver wouldn't check whether `total_supply` was accidentally modified.

## Dead Transition Detection

During verification, Verun also checks whether each transition's preconditions can ever be satisfied. If no valid state exists where the preconditions are true, the transition is unreachable:

```
[WARN] transition 'emergency_stop' has unsatisfiable preconditions (dead transition)
```

This catches design errors where a combination of invariants and preconditions makes a transition impossible to trigger.

## Control Flow

Transitions support `if/else` branching for conditional logic:

```verun
transition adjust(delta: int) {
    where { delta >= -50 && delta <= 50 }
    if x + delta > 100 {
        x = 100
    } else {
        if x + delta < 0 {
            x = 0
        } else {
            x = x + delta
        }
    }
}
```

The verifier encodes both branches and proves invariants hold regardless of which path is taken.

## Assertions

The `assert` statement adds a constraint inside a transition body. It tells the verifier "this condition must be true at this point":

```verun
transition swap(i: int, j: int) {
    where {
        i >= 0, i < top,
        j >= 0, j < top
    }
    assert i >= 0
    assert j >= 0
    // ... swap logic
}
```

In generated code, assertions become runtime checks (`assert!` in Rust, `require()` in Solidity, thrown errors in TypeScript). In verification, they add constraints to the solver's model.

## Putting It Together

Here's how the pieces interact in a complete spec:

```verun
enum Phase {
    Active,
    Paused,
    Closed
}

state Escrow {
    deposited: int
    released: int
    phase: Phase

    invariant conservation {
        released <= deposited
    }

    invariant non_negative {
        deposited >= 0 && released >= 0
    }

    init {
        deposited = 0,
        released = 0,
        phase = Phase::Active
    }

    transition deposit(amount: int) {
        where {
            amount > 0,
            phase == Phase::Active
        }
        deposited += amount
        ensure {
            deposited == old(deposited) + amount
        }
    }

    transition release(amount: int) {
        where {
            amount > 0,
            amount <= deposited - released,
            phase == Phase::Active
        }
        released += amount
        ensure {
            released == old(released) + amount
        }
    }

    transition pause() {
        where { phase == Phase::Active }
        phase = Phase::Paused
    }

    transition close() {
        where { phase == Phase::Paused }
        phase = Phase::Closed
    }
}
```

Verification proves:
1. The init state satisfies `conservation` and `non_negative`.
2. `deposit` preserves both invariants for all positive amounts.
3. `release` preserves both invariants — critically, the precondition `amount <= deposited - released` ensures `released` never exceeds `deposited`.
4. `pause` and `close` preserve both invariants (they don't touch numeric fields).
5. All transitions are reachable.
6. All postconditions hold.
