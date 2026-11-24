# OpenSequoia public incremental automation

## Locations and branch

- Public automation clone: `C:/Users/deepa/OneDrive/Dokumen/deepseek-harness/default-workspace/opensequoia-automation`.
- Read-only development source: `C:/Users/deepa/OneDrive/Dokumen/ChatGPT/opensequoia`.
- GitHub remote: `origin`, `https://github.com/ahjshine/opensequoia`.
- Local development remote: `source`.
- Publish branch: `main` only.
- Git uses the `openssl` HTTP backend because the host Schannel backend failed.

## Run contract

Each scheduled run publishes at most one coherent batch of existing development work. It must not invent empty commits or implement new features merely to satisfy a commit count. If no safe, coherent batch exists, make no commit and report why.

1. Work only in the public automation clone. Never edit, stage, commit, clean, reset, or rebase the development source or its crew worktrees.
2. Inspect `TASKS.md`, current source/crew branches, and available uncommitted changes. Prefer completed, reviewed source commits; otherwise select a self-contained pending batch that is not actively changing.
3. Do not mix unrelated subsystems. Preserve the public branch's MIT licensing decision. `LICENSE`, the root Cargo license, the README licensing wording, and newly generated package notices are MIT; historical evidence keeps its recorded notices.
4. Never weaken tests, security gates, input restrictions, or the Rust-engine boundary. Remote content stays disabled until the specified sandbox/TLS/origin gates pass.
5. Run meaningful focused checks for the selected files. For Rust changes, run `cargo fmt --all -- --check`, relevant tests, and `cargo test --workspace --locked` when practical; run Clippy for unsafe/platform changes. Do not claim checks that were not run. Windows Job/process tests can fail with `UNEXPECTED_ERROR` under harness confinement because child PowerShell is restricted; treat that as inconclusive, use an approved unrestricted Windows run for diagnosis, and never skip or weaken the test to publish. If shared temporary/helper files race, use an isolated `TEMP`/`TMP` under `target` and rerun the complete suite with `-- --test-threads=1`; this serializes execution without skipping coverage.
6. Scan staged changes for credentials, personal browsing data, and unintended local artifacts. Do not publish secrets.
7. Record the source revision, selected files, checks, limitations, and outcome in `evidence/automation/` without credentials or personal browsing data. Include that report in the same commit.
8. Use a descriptive commit message. Fetch `origin/main` before pushing. Push only a fast-forward with `git push origin main`; never force-push or overwrite remote work. If the remote moved or the batch is uncertain, stop safely.
9. Leave the automation clone clean after a successful push. If a check fails, do not commit or push the defective batch.

The schedule uses hourly deliveries from 08:00 through 17:00 Asia/Calcutta. Runs are serialized through one scheduled session so two runs do not write the clone concurrently.
