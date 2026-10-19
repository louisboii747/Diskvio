import type { ReactNode } from 'react'
import {
  HardDrive,
  Layers2,
  Usb,
  Settings2,
  RefreshCw,
  ShieldCheck,
  ChevronRight,
  CircleHelp,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { pageTitles, type Page } from '@/models/navigation'
const navigation = [
  { id: 'overview', icon: HardDrive },
  { id: 'partitions', icon: Layers2 },
  { id: 'usb', icon: Usb },
  { id: 'settings', icon: Settings2 },
] as const

interface Props {
  page: Page
  navigate: (page: Page) => void
  refresh: () => void
  loading: boolean
  status: string
  nativeStatus: string
  children: ReactNode
}
export function AppShell({
  page,
  navigate,
  refresh,
  loading,
  status,
  nativeStatus,
  children,
}: Props) {
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">
            <HardDrive size={21} />
          </span>
          <span>
            Diskvio<span className="brand-version">2.0</span>
          </span>
        </div>
        <div className="sidebar-label">WORKSPACE</div>
        <nav aria-label="Main navigation">
          {navigation.map(({ id, icon: Icon }) => (
            <Button
              key={id}
              variant="ghost"
              className={cn('nav-item', page === id && 'nav-active')}
              aria-label={pageTitles[id]}
              title={pageTitles[id]}
              aria-current={page === id ? 'page' : undefined}
              onClick={() => navigate(id)}
            >
              <Icon size={17} />
              <span>{pageTitles[id]}</span>
            </Button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <ShieldCheck size={18} />
          <div>
            <strong>Safe by design</strong>
            <p>No disk operations enabled</p>
          </div>
        </div>
        <div className="sidebar-build">
          MILESTONE 01 <span>FOUNDATION</span>
        </div>
      </aside>
      <div className="workspace">
        <header className="toolbar">
          <div className="breadcrumb">
            <span>Workspace</span>
            <ChevronRight size={14} />
            <strong>{pageTitles[page]}</strong>
          </div>
          <div className="toolbar-actions">
            <span className="demo-chip">DEMO</span>
            <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>
              <RefreshCw size={14} />
              <span>Refresh demo data</span>
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="About Diskvio"
              onClick={() => navigate('settings')}
            >
              <CircleHelp size={17} />
            </Button>
          </div>
        </header>
        <main id="main-content" tabIndex={-1}>
          {children}
        </main>
        <footer className="statusbar">
          <span className="status-message" role="status">
            <span className="status-dot" />
            {status}
          </span>
          <span>{nativeStatus}</span>
        </footer>
      </div>
    </div>
  )
}
