# Building

openXplane is a Cargo workspace (Rust edition 2024, current stable toolchain).

```sh
cargo build --release          # the application: target/release/openxplane
cargo test --workspace         # unit tests and the comparison with recorded originals
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

- **Linux** needs the usual Vulkan and windowing development packages (`libx11`, `libwayland`,
  `libxkbcommon`, `libvulkan`); CI installs them on Ubuntu.
- **macOS** uses Metal and needs only the Xcode command line tools.
- **Windows** uses DirectX 12 or Vulkan and the MSVC toolchain.

`scripts/build-linux.sh`, `scripts/build-macos.sh` and `scripts/build-windows.cmd` build a release and package it
the way the nightly workflow does. The workspace layout is described in [ARCHITECTURE.md](ARCHITECTURE.md).

## Tests that need an X-Plane installation

Some tests read content from a local copy in `Xplane12/` (git-ignored) and skip themselves without one.
The files in `crates/xp-app/tests/data` were produced from the original executable; to regenerate them you
need the reference executable and Python 3 with `unicorn` and `numpy` (`pip install unicorn numpy`):

```sh
python3 tools/gen_control_surface_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/control_surface.txt
```

Each generator names its output file in its header; [research/VERIFICATION.md](../research/VERIFICATION.md)
explains the method. Disassembly needs `llvm-objdump` on the path.
