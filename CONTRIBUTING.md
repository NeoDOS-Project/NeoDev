# Contributing to NeoDev

We welcome contributions! Here's how to get started.

## Development Setup

```bash
git clone https://github.com/NeoDOS-Project/NeoDev.git
cd NeoDev
cargo build --release
```

## Code Style

- Follow Rust standard formatting (`rustfmt`)
- Use `cargo clippy` to check for lint issues
- Keep error messages user-facing and bilingual (English/Spanish) where appropriate

## Pull Request Process

1. Create a feature branch from `develop`
2. Write clear commit messages following conventional commits
3. Ensure the project compiles: `cargo build --release`
4. Test with your NeoDOS checkout: `cargo run -- --neodos-path /path/to/NeoDOS <command>`
5. Open a PR against `develop`

## Adding a New VM Backend

1. Create `src/vmm/mybackend.rs`
2. Implement the `HypervisorBackend` trait
3. Register it in `src/vmm/mod.rs::create_backend()`
4. No CLI changes needed

## Reporting Issues

Report bugs at https://github.com/NeoDOS-Project/NeoDev/issues
