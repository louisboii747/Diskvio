import { beforeEach, expect, it, vi } from 'vitest'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getAppInfo, subscribeToMenu } from './native'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))
beforeEach(() => vi.clearAllMocks())
it('does not invoke or register listeners in browser preview', async () => {
  vi.mocked(isTauri).mockReturnValue(false)
  expect(await getAppInfo()).toBeNull()
  const stop = await subscribeToMenu(vi.fn())
  stop()
  expect(invoke).not.toHaveBeenCalled()
  expect(listen).not.toHaveBeenCalled()
})
it('invokes the real allowlisted Rust command and propagates failures', async () => {
  vi.mocked(isTauri).mockReturnValue(true)
  vi.mocked(invoke).mockRejectedValueOnce(new Error('offline'))
  await expect(getAppInfo()).rejects.toThrow('offline')
  expect(invoke).toHaveBeenCalledWith('get_app_info')
})
it('accepts only known menu actions', async () => {
  vi.mocked(isTauri).mockReturnValue(true)
  const callback = vi.fn()
  vi.mocked(listen).mockImplementation(async (_event, handler) => {
    handler({ event: 'diskvio://menu', id: 1, payload: 'settings' })
    handler({ event: 'diskvio://menu', id: 2, payload: 'format-disk' })
    return vi.fn<() => void>()
  })
  await subscribeToMenu(callback)
  expect(callback).toHaveBeenCalledExactlyOnceWith('settings')
})
