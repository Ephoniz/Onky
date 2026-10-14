## Summary

## How to test

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo test -p audio-program-engine --release --lib reference_session_cpu_budget -- --nocapture`
- [ ] `cargo deny check`
