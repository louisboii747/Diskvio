# Diskvio

<p align="center">
  <b>Disk management, without the guesswork.</b>
</p>

<p align="center">
  A cross-platform desktop utility for exploring disks and partitions, managing removable drives,
  and writing bootable USB media — with a focused interface and safety-first operations.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Status-early%20development-64748b" alt="Early development">
  <img src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&amp;logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/React-19-149ECA?logo=react&amp;logoColor=white" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-6-3178C6?logo=typescript&amp;logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&amp;logoColor=white" alt="Rust">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-targeted-0078D4?logo=windows&amp;logoColor=white" alt="Windows targeted">
  <img src="https://img.shields.io/badge/macOS-targeted-111111?logo=apple&amp;logoColor=white" alt="macOS targeted">
  <img src="https://img.shields.io/badge/Linux-targeted-FCC624?logo=linux&amp;logoColor=black" alt="Linux targeted">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-22c55e" alt="MIT licence"></a>
  <a href="https://github.com/louisboii747/Diskvio/stargazers"><img src="https://img.shields.io/github/stars/louisboii747/Diskvio?style=flat" alt="GitHub stars"></a>
</p>

---

> **Project status:** Milestone 2 implements real, read-only hardware discovery and an explicitly selectable Demonstration Mode. Windows discovery and the actual Tauri WebView have been verified without administrator rights. **Every disk-changing operation remains disabled.** macOS/Linux adapters are implemented; native host runtime verification remains outstanding.

## What works today

- Compact Drives, Partitions, USB Writer and Settings pages with plain-language labels. All page content fits the supported desktop window without scrolling.
- Live physical disk discovery, optional filesystem/volume metadata, exact partition offsets and stable read-only device selection. Three labelled sample disks remain available in Demonstration Mode.
- Byte-proportional partition maps, including unallocated space and readable tables for small system partitions.
- A theme picker with System, Light, Dark, Ocean, Forest and Sand; saved preferences, keyboard navigation and reduced-motion support.
- Real Rust `list_disks` and `get_app_info` commands connected through asynchronous Tauri IPC.
- Native menus with live refresh, navigation, Settings, About and Quit; unavailable operations are disabled.
- Shared Rust domain models, exact decimal-string byte transport, validation, platform adapters and an inactive non-privileged helper.
- Frontend/Rust tests, browser accessibility checks and a cross-platform CI configuration.

Partitions are inspectable and USB candidates are identified, with no target automatically selected. Image selection, formatting, deletion, resizing, writing, mount/unmount and eject are **Not Yet Available**. Refresh re-enumerates the selected source and preserves a matching physical selection. Live errors never fall back to samples. Browser preview uses labelled demo data and cannot select Live Mode. Desktop Settings remembers the source preference separately from themes.

| Platform | Read-only discovery | Limitations |
| --- | --- | --- |
| Windows | Native WMI through the maintained `wmi` Rust bindings: `MSFT_Disk`, `MSFT_Partition`, `MSFT_Volume`; unprivileged `Win32_*` compatibility fallback | Optional protected metadata stays unknown. No PowerShell is launched by the app. |
| macOS | Fixed `/usr/sbin/diskutil list -plist physical`, `info -plist` and `apfs list -plist` | APFS volumes remain children of backing partitions; shared usage is not summed. Multi-store APFS and platform runtime need host verification. |
| Linux | Fixed absolute `lsblk --json --bytes --tree --output ...` plus sysfs physical-device filtering | Loops, device mapper and virtual disks are excluded. Requires util-linux with the requested columns; inaccessible filesystem usage remains unknown. |

Disk/partition/volume distinctions, identity fallbacks and permission limits are detailed in [Architecture](docs/ARCHITECTURE.md). Unsupported layouts do not become invented unallocated space. Full serials and device paths are omitted by the opt-in diagnostic: `cargo run -p diskvio-platform --example discover --locked`.

## Design principles

1. **Function over decoration.** Readable data, compact layouts and restrained visual hierarchy.
2. **Never surprise the user.** Show the physical target and consequences before any future change.
3. **Default to read-only.** Discovery and inspection must remain unprivileged.
4. **Respect each OS.** Shared UI with native menus and explicit platform capabilities.
5. **No pretend success.** Simulated devices are labelled; unavailable work cannot report success.

## Technology

| Layer         | Implementation                                                     |
| ------------- | ------------------------------------------------------------------ |
| Desktop shell | Tauri 2; Rust 2.12.4 / CLI 2.12.3 in current lockfiles             |
| Interface     | React 19, TypeScript, Vite                                         |
| Design system | Tailwind CSS v4, official shadcn/ui Radix Nova integration, Lucide |
| State         | React state/hooks; no extra global store                           |
| Disk domain   | Shared Rust crates in one Cargo workspace                          |
| Tests         | Vitest, React Testing Library, Playwright, axe, Rust unit tests    |
| Packaging     | Windows MSI/NSIS built locally; unsigned development artifacts     |

The frontend follows the [official shadcn Vite integration](https://ui.shadcn.com/docs/installation/vite). Menus use [Tauri 2 native menu APIs](https://v2.tauri.app/learn/window-menu/), not an HTML imitation of a system menu bar.

## Development tools

- Node.js **24 LTS** and **pnpm 12.14.0** (`npm install --global pnpm@12.14.0`).
- Current stable Rust via [rustup](https://rustup.rs/), including `rustfmt` and `clippy`.
- Git and the platform prerequisites below.
- Google Chrome for optional browser/e2e checks (the Playwright config uses the installed `chrome` channel).

See the current [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) if your OS packages differ.

### Windows

Install Visual Studio 2022 Build Tools with **Desktop development with C++**, the MSVC x64/x86 toolchain and a Windows SDK. Install Microsoft Edge WebView2 Runtime if it is absent. Use the Rust MSVC host toolchain (`stable-x86_64-pc-windows-msvc`).

The application itself requires no administrator privileges. If PowerShell blocks the package-manager script, use `pnpm.cmd` instead of `pnpm`; no execution-policy change is necessary.

### macOS

Install Xcode Command Line Tools with `xcode-select --install`, then Node, pnpm and stable Rust. Build on macOS for native macOS artifacts. Signing/notarization is a future release task. Menu implementation is present, but its macOS runtime behavior has not been verified on this Windows development host.

### Linux

For Ubuntu 24.04/Debian-family development environments:

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev
```

Install equivalent packages on other distributions, plus Node, pnpm, stable Rust and a working desktop/font environment. Linux compilation and GUI behavior have not been verified locally.

## Install, develop and build

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm dev             # real Tauri desktop app
pnpm build           # production frontend only
pnpm build:desktop   # native executable and installers for the host OS
```

Equivalent native commands from the app directory:

```sh
cd apps/desktop
pnpm tauri dev
pnpm tauri build
```

`pnpm --filter @diskvio/desktop dev` starts only the browser preview at `http://127.0.0.1:1420`. Native menus/IPC require the Tauri app. Port 1420 must be free before starting a new development server.

Workspace builds go to the **root** `target/` directory. Windows output:

```text
target/release/diskvio.exe
target/release/bundle/msi/Diskvio_2.0.0_x64_en-US.msi
target/release/bundle/nsis/Diskvio_2.0.0_x64-setup.exe
```

These are unsigned development builds, not production releases. Installer execution and clean-machine installation have not been tested.

## Checks

```sh
pnpm check
pnpm lint
pnpm test
pnpm build
cargo fmt --all --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
pnpm --filter @diskvio/desktop test:e2e
```

Windows live verification observed **five physical disks and fourteen partitions**, matching independent read-only Windows inventory. The real Tauri WebView passed live discovery, source switching, refresh/selection, USB candidate filtering, theme persistence and disabled-action checks. See [the verification record](docs/VERIFICATION.md) for current automated counts, build results and platform limitations. Native menu click-through and installer execution remain separate manual checks.

The GitHub Actions matrix is configured for Windows/macOS/Linux. Its remote status is separate from local verification.

## Repository layout

```text
Diskvio/
├── .github/workflows/ci.yml
├── apps/desktop/
│   ├── src/
│   │   ├── components/          # Shell, shared states, shadcn primitives
│   │   ├── features/            # Disks, partitions, USB, settings
│   │   ├── fixtures/            # Demonstration devices
│   │   ├── hooks/               # Async state and theme
│   │   ├── lib/                 # Capacity/layout helpers
│   │   ├── models/              # Typed frontend contracts
│   │   ├── services/            # Fixtures and native IPC
│   │   └── test/                # Vitest setup; tests live beside code
│   ├── src-tauri/               # Rust shell, menus, capabilities, icons
│   ├── e2e/                    # Browser and optional native smoke checks
│   └── public/                 # Local brand asset
├── crates/
│   ├── diskvio-core/           # Pure models and validation
│   ├── diskvio-platform/       # Default-deny platform boundary
│   └── diskvio-helper/         # Inactive, non-privileged placeholder
├── assets/branding/            # Original icon source
├── docs/                       # Architecture, safety, verification
├── Cargo.toml / Cargo.lock
├── package.json / pnpm-lock.yaml
└── pnpm-workspace.yaml
```

Read [Architecture](docs/ARCHITECTURE.md) and [Safety](docs/SAFETY.md) before extending the native backend.

## Roadmap

- [x] Repository structure, README, licence and ignore rules
- [x] Tauri 2 + React + TypeScript application foundation
- [x] Accessible disk-list and partition-layout interface using explicitly marked fixtures
- [x] Functional navigation, device selection and a persisted theme picker
- [x] Rust models, validation, platform stubs and non-privileged helper placeholder
- [x] Native menu implementation and frontend event routing
- [x] Windows executable/installer builds and automated check configuration
- [ ] Native menu click-through verification on Windows, macOS and Linux
- [ ] Native file dialogs and image validation
- [x] Read-only discovery adapters for Windows, macOS and Linux; Windows hardware/runtime verified
- [ ] macOS/Linux hardware and native desktop runtime verification
- [ ] Strong device identity revalidation, capability checks and privilege boundaries
- [ ] Carefully tested destructive disk and removable-media operations
- [ ] Signed releases and clean-machine packaging verification

**Milestone 3 recommendation:** strengthen identity evidence and per-device capability reporting, expand host-based discovery validation and add read-only image inspection. Keep destructive operations disabled until their independent safety gates and platform tests are reviewed.

## Licence

Diskvio is licensed under the [MIT License](LICENSE).

---

<p align="center">If this project interests you, consider starring the repository to follow its development.</p>
