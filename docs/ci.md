# Linting and testing

[Lint and test](../.github/workflows/ci.yml) runs on every push to `main`, on pull requests targeting `main`, and on manual dispatch. A failing check fails its job. Jobs have timeouts, read-only repository permissions and no signing credentials. Pull request updates cancel obsolete runs; pushes to `main` keep their own runs.

## Platform coverage

| Job | Checks |
| --- | --- |
| Rust | Formatting, all-target checks, Clippy with warnings denied, workspace tests and documentation tests on macOS Apple Silicon, macOS Intel, Windows x64 and Linux x64. |
| macOS | SwiftLint, Swift presentation and contract tests, Debug Xcode unit tests, and a universal Apple Silicon/Intel Release build with the actual Rust static libraries. Swift and C compiler warnings fail the build. |
| Swift Intel | Presentation and contract tests execute on an Intel Mac, with Swift compiler warnings denied. |
| Windows x64 | .NET recommended analyzers and code style checks with warnings denied; Debug and Release WinUI/MSIX builds with the actual Rust DLL; managed fixtures and read-only native inventory/capability tests in both configurations. |

The Xcode project requires macOS 27. The native macOS job uses GitHub's [`xcode-27` runner](https://github.com/actions/runner-images/blob/main/images/macos/xcode-27-arm64-Readme.md), currently a preview image. GitHub's Intel image runs macOS 26, so Intel native app execution is not covered. Intel Rust and Swift model tests execute on that host, and Xcode 27 builds the Intel app as part of the universal Release build. Linux checks shared Rust portability; there is no Linux frontend.

Windows uses the [`windows-2025` image with Visual Studio 2026](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-VS2026-Readme.md), .NET 10 and the Windows SDK. MSIX packages build without signing or deployment. Native integration tests enumerate disks and query capabilities. Disk mutations remain fixture-based. External USB operations and UI interaction require separate manual verification.

SwiftLint 0.65.1 is pinned and its download checksum is verified. The check script downloads it on first use and caches it in `target/check-tools/`. The rules focus on correctness without imposing a repository-wide formatting rewrite. Rust dependencies use `Cargo.lock`. CI retains Xcode and Windows test results for seven days, including results produced before a failure. The template UI launch/performance tests are excluded from the Xcode unit test run. The actionlint configuration recognizes the official preview runner label until actionlint includes it in its built-in runner list.

## Commit checks

Install Python 3 and the prerequisites for your host app described in [README](../README.md). On macOS, Xcode 27 and `curl` are required; the script installs the pinned SwiftLint locally. `DISKVIO_SWIFTLINT` can override its executable path. On Windows, use a Developer PowerShell for Visual Studio 2026 so `msbuild`, Rust and .NET are available.

Enable the tracked hook once per clone, from the repository root:

```sh
git config --local core.hooksPath .githooks
```

Every commit then checks staged whitespace and runs Rust plus the host platform's lint, tests and Debug/Release builds. The hook checks a temporary copy of the staged files, including partially staged files, and removes that copy afterwards. It does not stash or alter source files. Windows commit checks use deterministic tests; read-only native tests also run in CI. Git hooks cannot be propagated automatically to new clones, so each developer must enable the hook. Server checks run when commits are pushed.

Run the same checks directly against the working tree:

```sh
python3 scripts/check.py
python3 scripts/check.py rust
python3 scripts/check.py swift
python3 scripts/check.py macos --universal
```

On Windows:

```powershell
python scripts/check.py
python scripts/check.py windows --native-integration
```

Universal macOS builds require both Rust targets: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`. Native command-line test results go in `target/ci/`, which is ignored by Git. Checks only report failures; they do not automatically fix source files. The shared `Diskvio` scheme explicitly selects the fixture-based unit test target.
