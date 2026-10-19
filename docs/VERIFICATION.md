# Milestone 1 verification

Environment: Windows x64, Node 24.16.0, pnpm 12.14.0, Rust/Cargo 1.99.0, Tauri Rust 2.12.4 / CLI 2.12.3. Lockfiles record exact dependency resolution.

## Automated checks

| Command                                                 | Result                                                                                                     |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `pnpm install`                                          | Passed                                                                                                     |
| `pnpm check`                                            | Passed: frontend and tooling TypeScript                                                                    |
| `pnpm lint`                                             | Passed: ESLint                                                                                             |
| `pnpm test`                                             | Passed: 14 tests                                                                                           |
| `pnpm build`                                            | Passed: production frontend                                                                                |
| `cargo fmt --all --check`                               | Passed                                                                                                     |
| `cargo check --workspace`                               | Passed on Windows                                                                                          |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed                                                                                                     |
| `cargo test --workspace`                                | Passed: 12 tests plus doc-test targets                                                                     |
| `pnpm --filter @diskvio/desktop test:e2e`               | Passed: fourteen Chrome runs                                                                               |
| `pnpm tauri build` from `apps/desktop`                  | Passed: Windows executable, MSI and NSIS installers                                                        |
| `pnpm tauri dev` from `apps/desktop`                    | Compiled and started the native process; a responding Diskvio window was observed through process metadata |
| `node e2e/native-smoke.mjs` from `apps/desktop`         | Passed against the actual Windows WebView with real Rust IPC                                               |

The final standalone release executable also passed `node e2e/native-smoke.mjs --packaged` with the development server stopped. Its bundled frontend rendered and Rust IPC connected under the production CSP.

The interface refresh also passed the packaged smoke check: all four pages fit the native window without scrolling, the Ocean theme survived a reload, and the app kept disk changes disabled. Screenshots of the theme picker, drive view and USB steps were inspected. The interface and native About metadata no longer show development-stage or version branding; internal package versions remain available for builds.

The browser suite checks all six theme choices at 1240 × 740 and 800 × 540. It checks each page for vertical and horizontal overflow and content cutoffs, plus selection, proportional partition widths, theme persistence, keyboard radio selection and system appearance changes. A separate 390 px browser check covers navigation and accessibility with readable scrolling content. Axe reports no WCAG A/AA violations for the tested screens. This is automated coverage, not a complete accessibility certification.

Rust covers capacity formatting, bounds/overflow, overlapping partitions, duplicate/missing identities, usage bounds, changed devices, the shared JSON fixture contract, disabled platform capabilities and application metadata. Frontend tests cover navigation, selection preservation, unavailable actions, native menu event routing/cleanup, refresh, loading/empty/error recovery, IPC failure and theme persistence.

Failures discovered and fixed: effect-driven redundant loading update, test typing/accessible-name mismatches, a Clippy collapsible condition, insufficient contrast on the first partition color and keyboard access to horizontally scrollable tables. The current shadcn CLI uses `--preset nova --base radix`; older base-color flags were rejected and replaced with supported options.

## Native runtime evidence and limits

The desktop inspection tool could not connect: `Computer Use native pipe is unavailable ... The system cannot find the file specified. (os error 2)`. We therefore did **not** verify menu clicks, About or Quit through OS UI automation.

A separate smoke run attached Playwright to the actual app's temporary WebView2 debug endpoint. The rendered Settings screen showed Windows, “Connected”, “Sample drives” and “Not available yet”. Menu listener registration succeeded without a permission error. This was the native webview, not a browser mock. A screenshot was inspected. The debug port and test profile are launch-time environment settings only; neither is enabled in the shipped configuration.

The native smoke script expects exactly one test webview at port 9223 and a local Vite server at port 1420. It is optional, not run by ordinary tests or CI. For a deliberate local Windows smoke run, close existing test instances, then:

```powershell
cd apps/desktop
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
$env:WEBVIEW2_USER_DATA_FOLDER = "$env:TEMP\diskvio-webview-smoke"
pnpm tauri dev
# In another terminal, from apps/desktop:
node e2e/native-smoke.mjs
```

Close the test app and remove those environment variables from the launching terminal afterward. Never enable a debugging port for ordinary or distributed builds.

For the release check, the final executable was launched with temporary WebView debug settings. The `--packaged` smoke option attaches to its `tauri.localhost` page without navigating to Vite. Both test instances and their temporary debug endpoints were closed after verification.

A dev restart initially found the previous Vite process still using port 1420. The native smoke run used a fresh local server. The default dev configuration intentionally fails on port conflicts instead of silently changing the IPC/frontend URL.

macOS and Linux compilation/runtime were not executed locally. The CI matrix is configured but its results must be checked independently. macOS application menu placement, services, Window/Help roles and shortcuts need a macOS host. Linux native menus need a desktop session.

Installers are unsigned development artifacts. Build success does not prove installer execution, signing, clean-machine installation, SmartScreen behavior or production release readiness. No installer was installed as part of verification.

## Manual follow-up

- On Windows, click native File refresh, View navigation/Settings, Help About and File Quit; verify disabled Disk/Partition/USB commands.
- On macOS, verify the top system menu ordering, native roles and Cmd shortcuts.
- On Linux, verify GTK/WebKit rendering and menu behavior.
- Test packaged installs on clean disposable environments before any release.
- No real devices were scanned or modified. Milestone 2 should begin with read-only discovery and identity/capability tests.
