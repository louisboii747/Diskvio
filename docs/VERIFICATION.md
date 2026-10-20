# Milestone 2 verification — 20 October 2026

Host: Windows x64, Node 24.16.0, pnpm 12.14.0, Rust/Cargo 1.99.0. The ordinary Windows user token was checked: administrator membership was **false**. No disk state was changed.

## Current automated checks

| Command | Result |
| --- | --- |
| `pnpm install --frozen-lockfile` | Passed; shared pnpm store access required running outside the filesystem sandbox |
| `pnpm check` / `pnpm lint` | Passed; no lint warnings |
| `pnpm test` | Passed: 26 deterministic frontend tests |
| `pnpm build` | Passed |
| `cargo fmt --all --check` | Passed |
| `cargo check --workspace --locked` | Passed on Windows |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed on Windows |
| `cargo test --workspace --locked` | Passed: 16 tests plus doc-test targets |
| `pnpm --filter @diskvio/desktop test:e2e` | Passed: 14 Chrome runs, all six themes at 1240 × 740 / 800 × 540, keyboard and narrow preview checks, axe WCAG A/AA checks |
| `pnpm tauri dev` | Real native process and WebView verified; the smoke run reused the existing local Vite server with a temporary config disabling duplicate server startup |
| `node e2e/native-smoke.mjs` | Passed against the actual native Windows WebView and real Rust `list_disks` IPC |
| `node e2e/native-smoke.mjs --packaged` | Passed against the bundled release EXE under its production CSP, with the dev app closed |
| `pnpm tauri build` | Passed: release EXE, MSI and NSIS; Windows Installer validation needed access outside the sandbox |

Tests cover exact string/u64 transport above JavaScript's safe integer range; optional serial/metadata/usage; invalid, duplicate, overlapping, overflowing and out-of-bounds layouts; provider fixture mapping for all three platforms; Windows compatibility-provider associations and permission-limited layouts; APFS backing-store relationships; Linux virtual-node filtering; live/demo source switching; structured errors/retry; empty inventory; concurrent stale refreshes; selection preservation/removal/replacement; missing filesystem metadata; USB eligibility and disabled actions. No arbitrary hardware is needed by ordinary tests or CI.

The compact UI initially overflowed at 800 × 540 after adding filesystem used/free values. Fixed by constraining accessible scrolling lists/tables and wrapping the compact usage column, without distorting partition widths or weakening the existing fit/accessibility assertions. The Playwright server now starts Vite directly so Windows process-tree teardown does not leave an extra package-manager server running.

## Observed Windows hardware

The read-only diagnostic and native app discovered these devices. Serial numbers, unique IDs and interface paths are intentionally omitted from this record.

| Device | Bus | Exact capacity (bytes) | Partitions |
| --- | --- | ---: | ---: |
| CT1000P3SSD8 | NVMe | 1000204886016 | 5 |
| MSI M461 2TB | NVMe | 2000398934016 | 6 |
| Samsung SSD 860 EVO 250GB | SATA | 250059350016 | 1 |
| WDC WD5000AVDS-63U7B1 | SATA | 500107862016 | 1 |
| Kingston DataTraveler 3.0 | USB | 61991813120 | 1 |

Independent **read-only** `Get-Disk` and `Get-Partition` output matched all five capacities, partition counts and all fourteen partition offsets/lengths. For example, the Windows data partition starts at byte 122683392 with length 998327189504; Kingston's partition starts at byte 1048576 with length 61989715968. NTFS and exFAT usage were obtained where Windows exposed them. Several non-Windows/system partitions lacked filesystem usage; the app reports unknown rather than zero. USB is a candidate only when non-system status is known, and remains unselected until the user chooses it; Write to USB stays disabled.

The sandbox blocks WMI (`0x80041003`) even though the same non-administrator user can query the native providers outside that isolation. No UAC, administrator token, application elevation, raw disk handle, privileged helper or modifying command was used. Production discovery uses native WMI, not the PowerShell comparison commands.

The real Tauri smoke check verified live inventory, exact byte-string IPC, source switching without mixed data, refresh selection continuity, USB candidates/no automatic target, disabled writing, theme persistence and page fit. It uses an explicitly enabled temporary localhost WebView debug port, never a production setting. Native menu event routing is unit-tested; OS menu clicks/About/Quit still need manual click-through. Physical unplug/replug was simulated through injected inventory changes, not performed on this host.

Opt-in diagnostic: `cargo run -p diskvio-platform --example discover --locked`. It prints names, capacities, geometry and filesystem figures but omits serials, IDs and device paths. Run it only when intentionally inspecting local hardware.

## Cross-platform and release limits

Windows, macOS and Ubuntu remain in the existing GitHub Actions matrix, running locked compile, strict Clippy, Rust/frontend tests and native release compilation. Adapter mapping fixtures for all platforms also run on Windows. Remote workflow results are separate from this local record and must be checked against the implementation commit; they are not claimed from fixtures or cross-compilation.

No macOS/Linux physical-host runtime testing was possible locally. APFS multi-store/container/snapshot behavior, diskutil metadata variations, Linux distribution/util-linux/permissions differences and native menus need host verification. Missing exact geometry stays unavailable. Whole-device filesystems do not currently have a partition-map presentation. RAID/dynamic/virtual topology is deliberately conservative and is not sufficient evidence for future writes. Missing serials and reused/cloned/path identities cannot prove physical replacement.

MSI/NSIS are unsigned development artifacts. Installer execution, clean-machine installation, signing/notarization and production release readiness remain unverified. The packaged release smoke command is `node e2e/native-smoke.mjs --packaged` after deliberately launching the release EXE with the temporary WebView settings described below. Close test processes and debug endpoints afterward.

---

# Historical Milestone 1 verification

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
