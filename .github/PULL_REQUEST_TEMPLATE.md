## Summary

<!-- 1-3 sentences: what does this PR do and why? -->

Closes #<!-- issue number -->

## Changes

<!-- Bullet list of key changes -->

-

## Test Evidence

```
$ cargo test
<!-- paste output or summary -->

$ npm test
<!-- paste output or summary -->
```

## Checklist

- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo test` passes
- [ ] `npm run build` succeeds
- [ ] `npm test` passes
- [ ] New Tauri commands registered in `invoke_handler`
- [ ] New CSS colors use theme tokens (not hardcoded hex)
- [ ] Schema changes have a migration in `storage/migrations.rs`
- [ ] PR title uses conventional commit format (`feat:`, `fix:`, `docs:`, etc.)
