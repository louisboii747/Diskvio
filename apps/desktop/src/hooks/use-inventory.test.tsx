import { act, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, expect, it, vi } from 'vitest'
import { useInventory, eligibleUsb } from './use-inventory'
import * as native from '@/services/native'
import * as devices from '@/services/devices'
import fixtures from '@/fixtures/disks.json'
import type { Disk } from '@/models/disk'
vi.mock('@/services/native', () => ({ isNative: vi.fn(), listDisks: vi.fn() }))
vi.mock('@/services/devices', () => ({ loadDemoDisks: vi.fn() }))
const disks = fixtures as Disk[]
beforeEach(() => {
  vi.clearAllMocks()
  localStorage.clear()
  vi.mocked(native.isNative).mockReturnValue(true)
  vi.mocked(native.listDisks).mockResolvedValue(structuredClone(disks))
  vi.mocked(devices.loadDemoDisks).mockResolvedValue(structuredClone(disks))
})
it('defaults native builds to live, keeps data during refresh, and preserves stable selection', async () => {
  const { result } = renderHook(useInventory)
  expect(result.current.source).toBe('live')
  await waitFor(() => expect(result.current.loading).toBe(false))
  act(() => result.current.select('demo-1'))
  let resolve!: (d: Disk[]) => void
  vi.mocked(native.listDisks).mockReturnValueOnce(
    new Promise((r) => {
      resolve = r
    }),
  )
  act(() => {
    void result.current.refresh()
  })
  expect(result.current.disks).toHaveLength(3)
  expect(result.current.loading).toBe(true)
  await act(async () => resolve(structuredClone(disks)))
  expect(result.current.selected?.id).toBe('demo-1')
})
it('clears removed or replaced selections without selecting a different drive', async () => {
  const { result } = renderHook(useInventory)
  await waitFor(() => expect(result.current.loading).toBe(false))
  vi.mocked(native.listDisks).mockResolvedValueOnce(disks.slice(1))
  await act(() => result.current.refresh())
  expect(result.current.selected).toBeNull()
  expect(result.current.notice).toMatch(/removed/)
  act(() => result.current.select('demo-1'))
  vi.mocked(native.listDisks).mockResolvedValueOnce([{ ...disks[1], serial: 'replacement' }])
  await act(() => result.current.refresh())
  expect(result.current.selected).toBeNull()
  expect(result.current.notice).toMatch(/identity changed/)
})
it('clears inventory on mode change, persists only native preference, and discards stale responses', async () => {
  let resolve!: (d: Disk[]) => void
  vi.mocked(native.listDisks).mockReturnValueOnce(
    new Promise((r) => {
      resolve = r
    }),
  )
  const { result } = renderHook(useInventory)
  act(() => result.current.changeSource('demo'))
  expect(result.current.disks).toEqual([])
  await waitFor(() => expect(result.current.loading).toBe(false))
  await act(async () => resolve([{ ...disks[0], name: 'Stale live response' }]))
  expect(result.current.disks[0].name).toBe('System Drive')
  expect(localStorage.getItem('diskvio-source')).toBe('demo')
})
it('ignores an older refresh that completes after a newer one', async () => {
  const { result } = renderHook(useInventory)
  await waitFor(() => expect(result.current.loading).toBe(false))
  let resolve!: (d: Disk[]) => void
  vi.mocked(native.listDisks).mockReturnValueOnce(
    new Promise((r) => {
      resolve = r
    }),
  )
  act(() => {
    void result.current.refresh()
  })
  vi.mocked(native.listDisks).mockResolvedValueOnce([disks[1]])
  await act(() => result.current.refresh())
  await act(async () => resolve(disks))
  expect(result.current.disks).toEqual([disks[1]])
})
it('reports structured live errors and never loads demo as a fallback', async () => {
  vi.mocked(native.listDisks).mockRejectedValueOnce({
    code: 'permissionDenied',
    message: 'Storage access denied',
  })
  const { result } = renderHook(useInventory)
  await waitFor(() => expect(result.current.error).toBe('Storage access denied'))
  expect(result.current.disks).toEqual([])
  expect(devices.loadDemoDisks).not.toHaveBeenCalled()
  await act(() => result.current.refresh())
  expect(result.current.disks).toHaveLength(3)
  vi.mocked(native.listDisks).mockResolvedValueOnce([])
  await act(() => result.current.refresh())
  expect(result.current.disks).toEqual([])
  expect(result.current.error).toBeNull()
})
it('browser previews ignore a saved live preference and refuse live discovery', async () => {
  vi.mocked(native.isNative).mockReturnValue(false)
  localStorage.setItem('diskvio-source', 'live')
  const { result } = renderHook(useInventory)
  await waitFor(() => expect(result.current.loading).toBe(false))
  act(() => result.current.changeSource('live'))
  expect(result.current.source).toBe('demo')
  expect(native.listDisks).not.toHaveBeenCalled()
})
it('USB candidates require known external USB and non-system status', () => {
  expect(eligibleUsb(disks[2])).toBe(true)
  for (const change of [
    { kind: 'unknown' as const },
    { system: null },
    { system: true },
    { connection: 'SATA' },
    { online: false },
    { readOnly: true },
  ])
    expect(eligibleUsb({ ...disks[2], ...change })).toBe(false)
})
