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
  <img src="https://img.shields.io/badge/React-planned-149ECA?logo=react&amp;logoColor=white" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-planned-3178C6?logo=typescript&amp;logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/Rust-planned-000000?logo=rust&amp;logoColor=white" alt="Rust">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-targeted-0078D4?logo=windows&amp;logoColor=white" alt="Windows targeted">
  <img src="https://img.shields.io/badge/macOS-targeted-111111?logo=apple&amp;logoColor=white" alt="macOS targeted">
  <img src="https://img.shields.io/badge/Linux-targeted-FCC624?logo=linux&amp;logoColor=black" alt="Linux targeted">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-22c55e" alt="MIT licence"></a>
  <a href="https://github.com/louisboii747/Diskvio/stargazers"><img src="https://img.shields.io/github/stars/louisboii747/Diskvio?style=flat" alt="GitHub stars"></a>
</p>

---

> **Project status:** Diskvio is being rebuilt from scratch. This repository currently contains the project foundation and architecture notes, **not a runnable app or production-ready disk operations**. Platform badges describe targets, not released support.

## What is Diskvio?

Diskvio is an open-source disk and partition utility planned for **Windows, macOS and Linux**. The goal is a dependable tool that makes storage devices easy to understand and provides clear, deliberate workflows for operations that can change or destroy data.

### Planned features

- **Disk overview:** physical devices, capacities, partitions, filesystems, mount points and connection details.
- **Partition management:** inspect, create, delete, format and resize partitions where the operating system and filesystem allow it.
- **USB media tools:** write compatible bootable images to removable devices and verify results where feasible.
- **Native desktop behaviour:** keyboard shortcuts, platform dialogs, context menus and a proper macOS application menu bar.
- **Transparent operations:** explicit capabilities, progress, actionable errors and confirmations before destructive changes.

Features above are **goals**, not currently implemented or guaranteed on every platform.

## Design principles

1. **Function over decoration.** Clear layouts, readable data, accessible controls and restrained motion.
2. **Never surprise the user.** Show which physical device will be affected, what will change and whether data can be lost.
3. **Default to read-only.** Discovery and inspection should not require elevated privileges.
4. **Respect each OS.** Share one UI while preserving native menus, dialogs and platform-specific capability reporting.
5. **No pretend success.** Show unavailable operations honestly and distinguish simulated fixtures from real hardware.

## Planned technology

| Layer | Technology |
| --- | --- |
| Desktop shell | Tauri 2 |
| User interface | React + TypeScript + Vite |
| Styling | Tailwind CSS and shadcn/ui |
| Icons | Lucide |
| Disk logic | Rust |
| Elevated operations | Small, carefully scoped platform-specific helper (planned) |
| Target platforms | Windows, macOS and Linux |

## Repository layout

```text
Diskvio/
├── .github/workflows/          # Future CI and release workflows
├── apps/
│   └── desktop/                # Future Tauri 2 + React application
├── crates/
│   ├── diskvio-core/           # Shared device models, validation and safety logic
│   ├── diskvio-platform/       # OS-specific discovery and disk capabilities
│   └── diskvio-helper/         # Planned narrow privileged operations helper
├── assets/
│   └── branding/               # Icons and other design assets
├── docs/
│   ├── ARCHITECTURE.md
│   └── SAFETY.md
├── .gitignore
├── LICENSE
└── README.md
```

Empty directories are retained with `.gitkeep` files until their implementations are added.

## Getting started

You can clone the foundation now:

```bash
git clone https://github.com/louisboii747/Diskvio.git
cd Diskvio
```

**There is no build command yet.** The next milestone is to scaffold the Tauri 2 application inside `apps/desktop/` and initialise the Rust workspace. After that, this section will include tested development and build steps.

For the intended boundaries between UI and disk operations, read [Architecture](docs/ARCHITECTURE.md) and [Safety principles](docs/SAFETY.md).

## Roadmap

- [x] Repository structure, README, licence and ignore rules
- [ ] Tauri 2 + React + TypeScript application foundation
- [ ] Accessible disk-list and partition-layout interface using explicitly marked fixture data
- [ ] Native macOS menu bar, keyboard shortcuts and platform dialogs
- [ ] Read-only real-device discovery on Windows, macOS and Linux
- [ ] Device identity revalidation, capability checks and privilege boundaries
- [ ] Carefully tested destructive disk and removable-media operations
- [ ] Packaging, CI and releases

## Licence

Diskvio is licensed under the [MIT License](LICENSE).

---

<p align="center">If this project interests you, consider starring the repository to follow its development.</p>
