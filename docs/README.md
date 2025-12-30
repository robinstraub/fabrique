# Documentation Guidelines

This documentation follows the [Diataxis framework](https://diataxis.fr/), which organizes
content into four distinct types based on user needs. Use these guidelines when writing or
reviewing documentation.

## Folder Structure

| Location | Diataxis Type | Purpose |
|--------|---------------|---------|
| `tutorials/` | Tutorials | Learning-oriented lessons |
| `concepts/` | Explanation | Understanding-oriented background |
| `guides/` | How-to Guides | Task-oriented problem solving |
| Rustdoc (generated) | Reference | Information-oriented technical descriptions |

## Tutorials (`tutorials/`)

> [diataxis.fr/tutorials](https://diataxis.fr/tutorials/)

**Purpose**: Help users *learn* through guided, hands-on experience.

**DO:**
- Show the destination upfront
- Deliver visible results early and often
- Use concrete, specific steps
- Ensure every step works reliably

**DON'T:**
- Explain concepts in detail (link to `concepts/` instead)
- Offer alternatives or options
- Assume prior knowledge

## How-to Guides (`guides/`)

> [diataxis.fr/how-to-guides](https://diataxis.fr/how-to-guides/)

**Purpose**: Help users *accomplish* a specific task.

**DO:**
- Focus on a single, well-defined goal
- Use action-oriented titles ("Using X", "Working with Y")
- Assume the reader knows what they want to achieve
- Provide conditional guidance ("If you need X, do Y")

**DON'T:**
- Teach or explain why (link to `concepts/` instead)
- Cover multiple unrelated tasks
- Include unnecessary reference material

## Explanation (`concepts/`)

> [diataxis.fr/explanation](https://diataxis.fr/explanation/)

**Purpose**: Help users *understand* how things work and why.

**DO:**
- Provide context, background, and rationale
- Make connections between concepts
- Discuss alternatives and trade-offs
- Use reflective language

**DON'T:**
- Include step-by-step instructions
- Document API signatures (use Rustdoc)
- Mix with how-to content

## Reference (Rustdoc)

> [diataxis.fr/reference](https://diataxis.fr/reference/)

Reference documentation is generated via **Rustdoc** from source code comments.
See the published API documentation at [docs.rs/fabrique](https://docs.rs/fabrique).

---

*See [diataxis.fr](https://diataxis.fr/) for the complete framework.*
