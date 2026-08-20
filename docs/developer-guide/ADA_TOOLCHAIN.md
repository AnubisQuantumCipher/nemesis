# Ada/SPARK Toolchain Bootstrap

## Observed local tools

The Phase 1 baseline was exercised on this host with:

- Alire 2.1.0, source commit `2681b313d08224014fa3f5e4f76e9c75347213d2`.
- `gnat_native` 14.2.1 / GCC 14.2.0 for `aarch64-apple-darwin23.6.0`.
- GPRbuild 24.0.1 selected by Alire.
- GNATprove reporting `0.0w`, Why3 1.6.0+git, Alt-Ergo 2.4.0, CVC5 1.1.2, and Z3 4.13.0.

These are observed tool identities, not compatibility claims for other releases.

## Build

```sh
./scripts/build_ada.sh
./build/bin/nemesis_kernel_tests
```

The wrapper keeps Alire's selected compiler, GPRbuild, `PATH`, and `GPR_PROJECT_PATH`, then removes only `LIBRARY_PATH` for the child GPRbuild process. On this macOS 26 host, Alire 2.1.0 prepends the selected GNAT runtime directory to `LIBRARY_PATH` while the GNAT 14.2.1 driver already emits the same runtime path. An ordinary `alr build` therefore produced two identical `LC_RPATH` load commands; dyld refused the executable at launch. The failure was reproduced, inspected with `otool -l`, isolated by rebuilding with individual Alire library variables removed, and resolved when only `LIBRARY_PATH` was absent. Do not replace this with post-link binary rewriting.

The GNAT toolchain currently emits deployment-target override warnings because this host/toolchain pair spans newer macOS tooling. The executable build and runtime gate remain authoritative; the warnings are retained rather than hidden. A future toolchain upgrade must remove the workaround only after the standard `alr build` binary launches and `otool -l` shows no duplicate runtime path.

## Proof

```sh
./scripts/prove_kernel.sh
```

The script runs GNATprove through the Alire environment against explicitly selected SPARK kernel bodies with `--mode=all`, `--level=2`, `--checks-as-errors=on`, and `--warnings=error`. Proof output names every established check. The gate does not claim whole-product formal verification or correctness of compiler, prover, runtime, operating system, or cryptographic libraries.

## Adding a trusted package

1. Update RFC-0001 if the package changes TCB membership or exceeds the configured source budget.
2. Write a failing executable hostile-control test.
3. Implement bounded SPARK declarations and bodies.
4. Add the body explicitly to `scripts/prove_kernel.sh`.
5. Run build, executable tests, and proof after the final source change.
6. Record exact proof output, assumptions, and unresolved checks in the phase receipt.

No proof suppression is permitted without an accepted trust-surface RFC naming the exact obligation and replacement evidence.
