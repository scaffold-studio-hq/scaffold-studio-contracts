<!--
Thanks for contributing to scaffold-studio-contracts. Keep the change scoped to a single issue
and fill in the sections below.
-->

## Summary

<!-- What does this change do, and why? -->

## Related issue

Fixes #

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] CI / tooling

## Verification

These are the checks CI runs in `.github/workflows/node.yml`; confirm the relevant ones pass
locally before requesting review.

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] `npm run lint`
- [ ] `npx prettier . --check`
- [ ] `npm run build`

## Contract behaviour changes

<!--
If this change alters anything under `contracts/`, describe the behaviour change, the storage or
interface impact, and the tests that cover it. Write "None" if there are no contract behaviour
changes.
-->

## Notes for reviewers

<!-- Anything else a reviewer should know, such as follow-up work or known limitations. -->
## Summary

<!-- What does this PR change and why? Link the issue it addresses. -->

Closes #

## Contract-behaviour changes

<!-- If this PR touches contracts/, describe the behavioural delta:
     new/changed entrypoints, authorization, storage keys, error codes. -->

- [ ] No contract behaviour changed, or described above.

## Verification

<!-- Check the commands you actually ran; all must pass before review. -->

- [ ] `cargo test --workspace`
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `npm run lint`
- [ ] `npm run build`
- [ ] `npx prettier . --check`

## Notes for reviewers

<!-- Migration concerns, deployment order, follow-up issues. -->
