# Fabrique Project Documentation for Claude

## Code Conventions

- **Always use ACME/Anvil themed examples**: Use Anvil, Hammer, etc. in all documentation and tests
- Avoid generic examples like User/Post - stick to the established ACME theme

## Documentation Requirements

- **All public APIs must be documented**: Every public trait, function, type, and method requires documentation
- **Match the existing documentation style**: New documentation must follow the same patterns and structure as existing code
  - Trait/type documentation: Summary line, blank line, detailed explanation
  - Method documentation: Action summary line, blank line, detailed explanation of behavior
  - Associated types: Single-line description
- **Keep documentation concise and consistent**: Follow the established tone and level of detail throughout the codebase

## Codegen Naming Conventions

- **Method naming pattern**: `generate_[kind]_[name]`
  - Examples: `generate_fn_all()`, `generate_struct_query_builder()`, `generate_type_connection()`
  - Kind: The type of code element being generated (`fn`, `struct`, `type`, `ident`, `constants`, `impl`, etc.)
  - Name: The specific identifier or purpose (e.g., `all`, `create`, `query_builder`)
- **Keep kinds consistent**: Use the same kind names across all codegen modules