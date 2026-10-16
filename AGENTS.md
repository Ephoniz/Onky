# Agent rules

Follow these on every change.

- Keep the code as simple as the plan allows. Prefer a small function with a clear name over a new abstraction.
- Use Conventional Commits. The subject says what changed. Add a body only when the reason is not obvious, and keep it to a few lines.
- Cover every behavior with tests. Do not skip or disable a test to get a green run.
- Do not change or delete a test that is already on `main` unless a human approves that in the pull request. Add a new test instead.
- Do not commit development-tool config, lockfiles, or ignore rules. Keep those files outside the repo.
