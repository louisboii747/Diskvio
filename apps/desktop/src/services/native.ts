import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Disk } from '@/models/disk'

export const isNative = () => isTauri()
export function listDisks(): Promise<Disk[]> {
  if (!isNative()) return Promise.reject(new Error('Live discovery requires the desktop app.'))
  return invoke<Disk[]>('list_disks')
}

export interface AppInfo {
  name: string
  version: string
  platform: string
  dataMode: 'live' | 'demo'
  capabilities: { discovery: boolean; partitionManagement: boolean; usbWriting: boolean }
}
export type MenuAction = 'refresh' | 'overview' | 'partitions' | 'usb' | 'settings'
const menuActions: readonly string[] = ['refresh', 'overview', 'partitions', 'usb', 'settings']

export async function getAppInfo(): Promise<AppInfo | null> {
  // A browser preview cannot prove native IPC; never fabricate a native response.
  return isTauri() ? invoke<AppInfo>('get_app_info') : null
}
export async function subscribeToMenu(onAction: (action: MenuAction) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return listen<string>('diskvio://menu', ({ payload }) => {
    if (menuActions.includes(payload)) onAction(payload as MenuAction)
  })
}
