import { useCallback, useEffect, useState } from 'react'
import { AppShell } from '@/components/layout/app-shell'
import { pageTitles, type Page } from '@/models/navigation'
import { DemoNotice } from '@/components/demo-notice'
import { ResourceState } from '@/components/resource-state'
import { DiskOverview } from '@/features/disks/disk-overview'
import { PartitionsScreen } from '@/features/partitions/partitions-screen'
import { UsbScreen } from '@/features/usb/usb-screen'
import { SettingsScreen } from '@/features/settings/settings-screen'
import { useResource } from '@/hooks/use-resource'
import { useTheme } from '@/hooks/use-theme'
import { loadDemoDisks } from '@/services/devices'
import { getAppInfo, subscribeToMenu } from '@/services/native'
import '@/index.css'

const descriptions: Record<Page, string> = {
  overview: 'A clear view of your storage, from physical device to partition.',
  partitions: 'Inspect how space is allocated across a device.',
  usb: 'Prepare bootable media with a clear, deliberate workflow.',
  settings: 'Make Diskvio feel at home on your desktop.',
}
export default function App() {
  const [page, setPage] = useState<Page>('overview')
  const [selectedId, setSelectedId] = useState('demo-0')
  const [refreshCount, setRefreshCount] = useState(0)
  const [menuError, setMenuError] = useState<string | null>(null)
  const { resource: devices, reload } = useResource(loadDemoDisks)
  const { resource: info, reload: reloadInfo } = useResource(getAppInfo)
  const { theme, setTheme } = useTheme()
  const navigate = useCallback((next: Page) => {
    setPage(next)
    requestAnimationFrame(() => document.querySelector<HTMLElement>('#page-title')?.focus())
  }, [])
  const refresh = useCallback(() => {
    reload()
    setRefreshCount((value) => value + 1)
  }, [reload])
  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | undefined
    subscribeToMenu((action) => {
      if (action === 'refresh') refresh()
      else navigate(action)
    })
      .then((stop) => {
        if (disposed) stop()
        else unlisten = stop
      })
      .catch((error: unknown) => {
        if (!disposed) setMenuError(String(error))
      })
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [navigate, refresh])
  const disks = devices.status === 'ready' ? devices.data : []
  const selected = disks.find((disk) => disk.id === selectedId) ?? disks[0]
  const nativeStatus =
    info.status === 'ready'
      ? info.data
        ? `Tauri connected · ${info.data.platform}`
        : 'Browser preview · native IPC unavailable'
      : info.status === 'error'
        ? 'Native connection unavailable'
        : 'Connecting to native backend…'
  const status =
    devices.status === 'loading'
      ? 'Loading local demo data…'
      : devices.status === 'error'
        ? 'Demo data unavailable'
        : refreshCount
          ? 'Demonstration data reloaded · no hardware scan'
          : 'Demonstration mode · no disk operations'
  return (
    <AppShell
      page={page}
      navigate={navigate}
      refresh={refresh}
      loading={devices.status === 'loading'}
      status={status}
      nativeStatus={nativeStatus}
    >
      <div className="page-heading">
        <div>
          <div className="eyebrow">DISKVIO WORKSPACE</div>
          <h1 id="page-title" tabIndex={-1}>
            {pageTitles[page]}
          </h1>
          <p>{descriptions[page]}</p>
        </div>
        <span className="milestone-label">MILESTONE 01</span>
      </div>
      {menuError && (
        <div role="alert" className="error-banner">
          Native menu connection failed: {menuError}. Use sidebar navigation.
        </div>
      )}
      {page !== 'settings' && <DemoNotice />}
      {page === 'settings' ? (
        <SettingsScreen theme={theme} setTheme={setTheme} info={info} retry={reloadInfo} />
      ) : devices.status === 'loading' ? (
        <ResourceState kind="loading" />
      ) : devices.status === 'error' ? (
        <ResourceState kind="error" message={devices.message} retry={reload} />
      ) : !selected ? (
        <ResourceState kind="empty" retry={reload} />
      ) : page === 'overview' ? (
        <DiskOverview
          disks={disks}
          selected={selected}
          select={setSelectedId}
          openPartitions={() => navigate('partitions')}
        />
      ) : page === 'partitions' ? (
        <PartitionsScreen disks={disks} selected={selected} select={setSelectedId} />
      ) : (
        <UsbScreen disks={disks} selectedId={selected.id} select={setSelectedId} />
      )}
    </AppShell>
  )
}
