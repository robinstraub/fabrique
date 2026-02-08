# Architecture

The query builder is the heart of Fabrique. Models, factories, and
convenience methods all funnel through it — it is the central engine
that everything else builds on. This is what makes Fabrique a toolkit
rather than an ORM: the model is a declaration interface that
provides ergonomics and abstraction, but the query builder does the
actual work.

Fabrique builds on [sqlx](https://github.com/launchbadge/sqlx) for
database access. sqlx handles connection pooling, parameter binding,
and SQL injection protection. Fabrique constructs structurally
correct queries, then delegates execution to sqlx.

## Two Layers

The query builder is split into two layers:

- **Layer 1 (SQL)** constructs valid SQL from string identifiers.
  It knows about SQL structure — `SELECT`, `WHERE`, `JOIN` — but
  nothing about your models. Table and column names are plain
  `&str`.

- **Layer 2 (Model)** wraps Layer 1 with model awareness. It
  resolves table names, column names, and types from your model
  declarations. This is the layer you interact with —
  `Product::query()` creates a Layer 2 builder.

Layer 2 forwards every operation to Layer 1 after resolving
identifiers. The split means Layer 1 guarantees valid SQL structure
independently, while Layer 2 adds type safety on top.

## Join Tracking

When you call `.join::<Order>()`, the query builder needs to
remember that `Order` is now part of the query — so it can validate
subsequent operations like `.r#where(Order::STATUS, ...)` or
`.select_as::<Order, _>()`.

Fabrique tracks this at the type level using a **heterogeneous list**
(HList): a recursive cons-list where each element can be a different
type. Joining `Order` to a `Product` query produces a type like
`Joined<Order, Joined<Product, ()>>`.

This structure generates heavy type signatures internally. The
`Contains` trait abstracts over the list to answer one question: "is
model `M` present in this query?" When it isn't, the compiler
produces a clear diagnostic:

```text
error: model `Order` is not joined in this query
  --> src/main.rs:12:5
   |
12 |     .r#where(Order::STATUS, "=", "shipped".to_string())
   |     ^^^^ this column requires its model to be joined
   |
   = note: add `.join::<Order>()` before using columns
           from `Order`
```

In practice, these type signatures rarely surface. The query builder
is consumed on execution — every method takes `self` by value, and
the builder is not `Clone`. As long as you build and execute a query
in a single chain (which is the natural usage), the full type lives
only inside the chain and never appears in your code.

## The Typestate Pattern

Each method on the query builder takes `self` by value and returns a
builder in a new state type. Valid transitions compile; invalid ones
don't — you cannot diverge from the logical SQL ordering.

The SELECT flow illustrates this:

```text
query ─→ join ─→ where ─→ order_by ─→ limit ─→ offset
   │       │       │ ↺       │           │        │
   │       │       ↓         ↓           ↓        ↓
   └───────┴──── get / first / first_or_fail ─────┘
```

Joins come before filters, filters before ordering, ordering before
pagination. Calling `.join()` after `.where()`, or `.set()` on a
SELECT query, won't compile — the method simply doesn't exist on
that state. INSERT and UPDATE follow the same principle with their
own state machines. See the
[API reference](https://docs.rs/fabrique) for the complete flows.
