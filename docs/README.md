# Documentation Guidelines

This document describes conventions for writing Fabrique documentation.

We follow the [Diataxis framework](https://diataxis.fr/) for content
organization, with additional conventions for Rust code examples.

## Diataxis Framework

| Location            | Type          | Purpose                           |
| ------------------- | ------------- | --------------------------------- |
| `tutorials/`        | Tutorials     | Learning-oriented lessons         |
| `concepts/`         | Explanation   | Understanding-oriented background |
| `cookbook/`         | How-to Guides | Task-oriented problem solving     |
| Rustdoc (generated) | Reference     | Information-oriented descriptions |

### Tutorials (`tutorials/`)

> [diataxis.fr/tutorials](https://diataxis.fr/tutorials/)

**Purpose**: Help users *learn* through guided, hands-on experience.

**DO:** Show the destination upfront, deliver visible results early,
use concrete steps, ensure every step works reliably.

**DON'T:** Explain concepts in detail, offer alternatives, assume
prior knowledge.

### How-to Guides (`cookbook/`)

> [diataxis.fr/how-to-guides](https://diataxis.fr/how-to-guides/)

**Purpose**: Help users *accomplish* a specific task.

**DO:** Focus on a single goal, use action-oriented titles that
combine the problem and the feature (e.g. "Keep Order History
Intact with Soft Deletes"), assume the reader knows what they want,
provide conditional guidance.

**DON'T:** Teach or explain why, cover multiple unrelated tasks.

### Explanation (`concepts/`)

> [diataxis.fr/explanation](https://diataxis.fr/explanation/)

**Purpose**: Help users *understand* how things work and why.

**DO:** Provide context and rationale, make connections, discuss
trade-offs.

**DON'T:** Include step-by-step instructions, document API
signatures.

### Reference (Rustdoc)

> [diataxis.fr/reference](https://diataxis.fr/reference/)

Reference documentation is generated via Rustdoc from source code
comments.

## Page Transitions

Every concept page ends with a `---` separator followed by a "Next"
link to the next concept in reading order. This guides readers
through the documentation without forcing them to go back to the
table of contents.

Tutorials link to concepts for deeper understanding, not to
cookbooks. Cookbooks are standalone recipes — they link back to
concepts for background when needed.

## Code Conventions

### Executable Examples

All code examples should be executable via `mdbook test`. Use the
`#[fabrique::doctest]` macro to set up an in-memory SQLite database:

```rust
# #[fabrique::doctest]
# async fn main(pool: Pool<Sqlite>) -> Result<(), fabrique::Error> {
let user = User::factory().create(&pool).await?;
assert_eq!(user.name, "Test User");
# Ok(())
# }
```

Hide the doctest wrapper with `#` prefix. The visible code should
focus on the feature being demonstrated.

### Formatting

**Section headers** — For complex examples, use 80-character comment
blocks to separate logical sections (Models, Service functions,
etc.). Not needed for simple examples.

```rust
// -----------------------------------------------------------
// Models
// -----------------------------------------------------------
```

**Omitted code** — Use `// --snip--` to indicate code that exists
but is not shown.

**Hidden code** — Use `#` prefix to hide boilerplate (imports,
struct definitions) that would distract from the main point.

**Line length** — Keep all lines under 80 characters, including
hidden code. Format struct definitions on multiple lines.

**Doc comments** — Use `///` on all public functions and structs:

```rust
/// A user who can place orders.
pub struct User { ... }

/// Fetches a user and all their orders.
pub async fn get_user_with_orders(...) { ... }
```

**Test comments** — Describe expected behavior, not mechanical
actions. Use impersonal phrasing:

```rust
// Good: get_user_pending_orders should only return pending orders
// Bad:  Call get_user_pending_orders and check the result
```

### Content Best Practices

**Explain design decisions** — When a choice might surprise readers,
explain the reasoning. Example: why `unit_price_cents` is stored in
`order_lines` even though products have a price (to capture the
price at purchase time).

**Use realistic examples** — Prefer domain-specific names (`User`,
`Order`) over generic ones (`Foo`, `Bar`). Use realistic values
(`"Wile E. Coyote"`, `4999` cents) over placeholders.

## Useful Commands

```bash
# Build the project
cargo build --features sqlite

# Run documentation tests
CARGO_MANIFEST_DIR=$PWD/docs mdbook test docs -L target/debug/deps

# Preview the documentation locally
mdbook serve docs
```
