# Crunchie Apps

A monorepo for applications powered by the **Crunchie** math engine.

## Installation

You can install the apps directly from this repository using `cargo`.

### From GitHub
```bash
cargo install --git https://github.com/Taugeshtu/crunchie-apps crunchie-pad
```

### From Local Source
If you have the repository cloned locally:
```bash
cargo install --path crunchie-pad
```

## Projects

*   **[crunchie-core](https://github.com/Taugeshtu/crunchie-core)**: The central Rust library. Handles parsing, topology normalization, and evaluation.
*   **[crunchie-pad](./crunchie-pad)**: A minimal "sticky notes" style desktop app for live math scratchpadding.

