import { Check, Monitor, ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { themes, type Theme } from '@/models/themes'
import type { Resource } from '@/hooks/use-resource'
import type { AppInfo } from '@/services/native'
import { isNative } from '@/services/native'
import type { InventorySource } from '@/models/disk'
const platformNames: Record<string, string> = { windows: 'Windows', macos: 'macOS', linux: 'Linux' }
export function SettingsScreen({
  theme,
  setTheme,
  info,
  retry,
  source,
  changeSource,
}: {
  theme: Theme
  setTheme: (theme: Theme) => void
  info: Resource<AppInfo | null>
  retry: () => void
  source: InventorySource
  changeSource: (source: InventorySource) => void
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
        <label className="source-picker">
          Device source
          <select value={source} onChange={(e) => changeSource(e.target.value as InventorySource)}>
            <option value="live" disabled={!isNative()}>
              Live · Read only
            </option>
            <option value="demo">Demonstration Mode</option>
          </select>
        </label>
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
                <dd>{source === 'live' ? 'Live devices' : 'Sample drives'}</dd>
              </div>
              <div>
                <dt>Drive changes</dt>
                <dd>Not available yet</dd>
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
            <p>Discovery is read only. You can't change or erase a drive.</p>
          </div>
        </div>
        <p className="license-copy">Free and open source · MIT License</p>
      </section>
    </div>
  )
}
