import { Check, Monitor, ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { themes, type Theme } from '@/models/themes'
import type { Resource } from '@/hooks/use-resource'
import type { AppInfo } from '@/services/native'
const platformNames: Record<string, string> = { windows: 'Windows', macos: 'macOS', linux: 'Linux' }
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
    <div className="settings-layout">
      <section className="panel settings-panel">
        <h2>Choose a theme</h2>
        <p>Pick the look you like. Diskvio remembers your choice.</p>
        <fieldset className="theme-picker">
          <legend className="sr-only">Theme</legend>
          {themes.map((option) => (
            <label
              key={option.id}
              className={`theme-choice${theme === option.id ? ' theme-selected' : ''}`}
            >
              <input
                type="radio"
                name="theme"
                value={option.id}
                checked={theme === option.id}
                onChange={() => setTheme(option.id)}
                aria-label={option.name}
              />
              <span
                className="theme-preview"
                data-theme={option.id === 'system' ? 'light' : option.id}
                aria-hidden="true"
              >
                <span className="preview-sidebar">
                  <i />
                  <i />
                  <i />
                </span>
                <span className="preview-content">
                  <i />
                  <span>
                    <i />
                    <i />
                    <i />
                  </span>
                </span>
                {option.id === 'system' && <Monitor size={16} />}
              </span>
              <span className="theme-label">
                <strong>{option.name}</strong>
                {theme === option.id && <Check size={14} />}
              </span>
              <span className="theme-description">{option.description}</span>
            </label>
          ))}
        </fieldset>
      </section>
      <section className="panel settings-panel about-panel">
        <h2>About Diskvio</h2>
        <p>See your drives and how they use space.</p>
        {info.status === 'loading' && <p role="status">Connecting to the desktop app...</p>}
        {info.status === 'error' && (
          <div className="connection-error" role="alert">
            <p>We couldn't connect to the desktop app.</p>
            <p className="error-detail">{info.message}</p>
            <Button variant="outline" onClick={retry}>
              Try again
            </Button>
          </div>
        )}
        {info.status === 'ready' &&
          (info.data ? (
            <dl className="about-details">
              <div>
                <dt>Running on</dt>
                <dd>{platformNames[info.data.platform] ?? info.data.platform}</dd>
              </div>
              <div>
                <dt>Desktop app</dt>
                <dd>Connected</dd>
              </div>
              <div>
                <dt>Drives shown</dt>
                <dd>Sample drives</dd>
              </div>
              <div>
                <dt>Drive changes</dt>
                <dd>
                  {Object.values(info.data.capabilities).some(Boolean)
                    ? 'Unavailable in this demo'
                    : 'Not available yet'}
                </dd>
              </div>
            </dl>
          ) : (
            <div className="browser-note">
              You're using the browser preview. Open the desktop app to use its menus.
            </div>
          ))}
        <div className="safety-note">
          <ShieldCheck size={20} />
          <div>
            <strong>Your files stay untouched</strong>
            <p>This demo uses sample drives. You can't change or erase a drive.</p>
          </div>
        </div>
        <p className="license-copy">Free and open source · MIT License</p>
      </section>
    </div>
  )
}
