import { Monitor, Moon, Sun, ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { Theme } from '@/hooks/use-theme'
import type { Resource } from '@/hooks/use-resource'
import type { AppInfo } from '@/services/native'
export function SettingsScreen({
  theme,
  setTheme,
  info,
  retry,
}: {
  theme: Theme
  setTheme: (theme: Theme) => void
  info: Resource<AppInfo | null>
  retry: () => void
}) {
  return (
    <div className="settings-stack">
      <section className="panel settings-panel">
        <h2>Appearance</h2>
        <p>Choose a theme for your workspace. Your preference is saved on this device.</p>
        <div className="theme-options" role="group" aria-label="Theme">
          {(
            [
              { id: 'light', icon: Sun, label: 'Light' },
              { id: 'dark', icon: Moon, label: 'Dark' },
              { id: 'system', icon: Monitor, label: 'System' },
            ] as const
          ).map(({ id, icon: Icon, label }) => (
            <Button
              key={id}
              variant={theme === id ? 'secondary' : 'outline'}
              aria-pressed={theme === id}
              onClick={() => setTheme(id)}
            >
              <Icon size={18} />
              {label}
            </Button>
          ))}
        </div>
      </section>
      <section className="panel settings-panel">
        <h2>About Diskvio</h2>
        <p>Disk management, without the guesswork.</p>
        {info.status === 'loading' && <p role="status">Connecting to the native application…</p>}
        {info.status === 'error' && (
          <div role="alert">
            <p>Native application information is unavailable: {info.message}</p>
            <Button variant="outline" onClick={retry}>
              Retry connection
            </Button>
          </div>
        )}
        {info.status === 'ready' &&
          (info.data ? (
            <dl className="about-details">
              <div>
                <dt>Version</dt>
                <dd>{info.data.version} · Milestone 1</dd>
              </div>
              <div>
                <dt>Native platform</dt>
                <dd>{info.data.platform}</dd>
              </div>
              <div>
                <dt>Backend connection</dt>
                <dd>Connected through Tauri IPC</dd>
              </div>
              <div>
                <dt>Data source</dt>
                <dd>Local demonstration fixtures</dd>
              </div>
              <div>
                <dt>Disk capabilities</dt>
                <dd>
                  {Object.values(info.data.capabilities).some(Boolean)
                    ? 'Unexpected capability response'
                    : 'All unavailable'}
                </dd>
              </div>
            </dl>
          ) : (
            <div className="guidance-note">
              Browser preview. Native application information and system menus are available only in
              the Tauri desktop app.
            </div>
          ))}
        <p className="license-copy">
          Open source · MIT License · Windows, macOS and Linux architecture
        </p>
      </section>
      <section className="action-panel">
        <ShieldCheck size={22} />
        <div>
          <strong>Default-deny safety model</strong>
          <p>
            No real-device discovery, disk writes, privilege escalation or arbitrary shell
            execution.
          </p>
        </div>
      </section>
    </div>
  )
}
