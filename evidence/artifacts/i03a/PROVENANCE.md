# I03a evidence artifacts

Five 1,043,456-byte PE files used by `evidence/verification/i03a-repro.py`.

| File | SHA256 | Origin |
|---|---|---|
| `recorded-opensequoia.exe` | `b8ae67c3b6839c1be1751bb3eb49088bf9999491b38284b58eb309e74cbc7499` | `opensequoia-crew/controller/target/i03a/release/opensequoia.exe`; source `54289fd` |
| `rebuilt-default1-opensequoia.exe` | `df0f8b9c1a55ac7cbb13db9abffcfe435b78d2d17e1f243bb3a6c9709f32e23c` | clean detached worktree at `54289fd`, fresh target, canonical command |
| `rebuilt-default2-opensequoia.exe` | `ed265d2f30cc9e2a884aa6f8a7fd5c679660baaa07ca7fed5074bfbb679ead1e` | second independent clean build, same command |
| `rebuilt-brepro1-opensequoia.exe` | `165af357529758fd51a54df676c629a60331637f3ceef2d436c467b6c3572d26` | clean build with `RUSTFLAGS='-C control-flow-guard=yes -C link-arg=/Brepro'` |
| `rebuilt-brepro2-opensequoia.exe` | `165af357529758fd51a54df676c629a60331637f3ceef2d436c467b6c3572d26` | second independent `/Brepro` build, byte-identical |

Toolchain `rustc 1.98.1 (48a229cea 2026-09-01)`. These are evidence artifacts, not release artifacts; none is signed.
