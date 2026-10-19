# Diskvio desktop

React + TypeScript + Vite frontend and Tauri 2 native shell.

From the repository root:

```sh
pnpm install
pnpm dev
pnpm build:desktop
```

From this directory:

```sh
pnpm tauri dev
pnpm tauri build
```

`pnpm dev` **in this directory** starts the browser preview only. Its footer explicitly reports that native IPC is unavailable. Use `pnpm tauri dev` to test the real desktop app.

See the [root README](../../README.md), [architecture](../../docs/ARCHITECTURE.md), [safety rules](../../docs/SAFETY.md) and [verification record](../../docs/VERIFICATION.md).
