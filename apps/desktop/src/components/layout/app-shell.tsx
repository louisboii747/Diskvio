import type { ReactNode } from 'react'
import { HardDrive, Layers2, Usb, Settings2, RefreshCw, ShieldCheck } from 'lucide-react'
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
  description: string
  navigate: (page: Page) => void
  refresh: () => void
  loading: boolean
  status: string
  nativeStatus: string
  children: ReactNode
}
export function AppShell({
  page,
  description,
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
          <span>Diskvio</span>
        </div>
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
              <Icon size={18} />
              <span>{pageTitles[id]}</span>
            </Button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <ShieldCheck size={19} />
          <div>
            <strong>Demo mode</strong>
            <p>Sample drives only</p>
          </div>
        </div>
      </aside>
      <div className="workspace">
        <header className="toolbar">
          <div className="page-heading">
            <h1 id="page-title" tabIndex={-1}>
              {pageTitles[page]}
            </h1>
            <p>{description}</p>
          </div>
          <Button
            variant="outline"
            size="sm"
            aria-label="Reload samples"
            onClick={refresh}
            disabled={loading}
          >
            <RefreshCw size={14} />
            <span>Reload samples</span>
          </Button>
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
