import { useCallback, useEffect, useState } from 'react'
import { AppShell } from '@/components/layout/app-shell'
import type { Page } from '@/models/navigation'
import { DemoNotice } from '@/components/demo-notice'
import { ResourceState } from '@/components/resource-state'
import { DiskOverview } from '@/features/disks/disk-overview'
import { PartitionsScreen } from '@/features/partitions/partitions-screen'
import { UsbScreen } from '@/features/usb/usb-screen'
import { SettingsScreen } from '@/features/settings/settings-screen'
import { useResource } from '@/hooks/use-resource'
import { useTheme } from '@/hooks/use-theme'
import { useInventory } from '@/hooks/use-inventory'
import { getAppInfo, subscribeToMenu } from '@/services/native'
import '@/index.css'

const descriptions: Record<Page, string> = {
  overview: 'Choose a drive to see its space and partitions.',
  partitions: 'A partition is a separate section of a drive.',
  usb: 'Put an operating system on a USB drive.',
  settings: 'Choose how Diskvio looks on your computer.',
}
export default function App() {
  const [page, setPage] = useState<Page>('overview')
  const [refreshCount, setRefreshCount] = useState(0)
  const [menuError, setMenuError] = useState<string | null>(null)
  const inventory = useInventory()
  const { disks, selected, source, refresh: reload, select } = inventory
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
  const nativeStatus =
    info.status === 'ready'
      ? info.data
        ? 'Desktop app connected'
        : 'Browser preview'
      : info.status === 'error'
        ? 'Desktop app disconnected'
        : 'Connecting to the desktop app...'
  const status = inventory.loading
    ? source === 'live'
      ? 'Discovering drives...'
      : 'Loading sample drives...'
    : inventory.error
      ? 'Refresh failed'
      : refreshCount
        ? source === 'demo'
          ? 'Sample drives reloaded'
          : 'Live inventory refreshed'
        : source === 'live'
          ? 'Live · Read only'
          : 'Demo mode'
  return (
    <AppShell
      page={page}
      description={descriptions[page]}
      navigate={navigate}
      refresh={refresh}
      loading={inventory.loading}
      status={status}
      source={source}
      nativeStatus={nativeStatus}
    >
      {menuError && (
        <div role="alert" className="error-banner">
          The app menu couldn't connect. You can use the buttons on the left.
          <span className="error-detail">{menuError}</span>
        </div>
      )}
      {page !== 'settings' && <DemoNotice source={source} />}
      {inventory.notice && <p role="status">{inventory.notice}</p>}
      {inventory.error && page !== 'settings' && disks.length > 0 && (
        <div role="alert" className="error-banner">
          Refresh failed. Showing the previous inventory, which may be stale. {inventory.error}{' '}
          <button onClick={refresh}>Try again</button>
        </div>
      )}
      {page === 'settings' ? (
        <SettingsScreen
          theme={theme}
          setTheme={setTheme}
          info={info}
          retry={reloadInfo}
          source={source}
          changeSource={inventory.changeSource}
        />
      ) : inventory.loading && disks.length === 0 ? (
        <ResourceState kind="loading" source={source} />
      ) : inventory.error && disks.length === 0 ? (
        <ResourceState kind="error" message={inventory.error} retry={refresh} source={source} />
      ) : disks.length === 0 ? (
        <ResourceState kind="empty" retry={refresh} source={source} />
      ) : !selected ? (
        <div className="panel">
          <p>Choose a drive to inspect.</p>
          <select aria-label="Drive" value="" onChange={(e) => select(e.target.value)}>
            <option value="" disabled>
              Choose a drive
            </option>
            {disks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </div>
      ) : page === 'overview' ? (
        <DiskOverview
          disks={disks}
          selected={selected}
          select={select}
          openPartitions={() => navigate('partitions')}
        />
      ) : page === 'partitions' ? (
        <PartitionsScreen disks={disks} selected={selected} select={select} />
      ) : (
        <UsbScreen key={source} disks={disks} source={source} />
      )}
    </AppShell>
  )
}
