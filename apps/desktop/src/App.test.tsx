import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import fixtures from '@/fixtures/disks.json'
import * as devices from '@/services/devices'
import * as native from '@/services/native'
import type { MenuAction } from '@/services/native'

vi.mock('@/services/devices', () => ({ loadDemoDisks: vi.fn() }))
vi.mock('@/services/native', () => ({ getAppInfo: vi.fn(), subscribeToMenu: vi.fn() }))
let menuAction: (action: MenuAction) => void
const stopListening = vi.fn()
beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(devices.loadDemoDisks).mockResolvedValue(
    structuredClone(fixtures) as Awaited<ReturnType<typeof devices.loadDemoDisks>>,
  )
  vi.mocked(native.getAppInfo).mockResolvedValue(null)
  vi.mocked(native.subscribeToMenu).mockImplementation(async (callback) => {
    menuAction = callback
    return stopListening
  })
})
const nav = (name: string) =>
  within(screen.getByRole('navigation', { name: 'Main navigation' })).getByRole('button', { name })
describe('Diskvio workspace', () => {
  it('labels demo data and preserves selected disk across navigation and refresh', async () => {
    const user = userEvent.setup()
    render(<App />)
    expect(screen.getByText('DEMO DATA / SIMULATED DEVICES')).toBeVisible()
    await user.click(await screen.findByRole('button', { name: 'Select Archive' }))
    expect(screen.getByRole('region', { name: 'Selected device' })).toHaveTextContent(
      'Crucial MX500',
    )
    await user.click(nav('Partitions'))
    expect(screen.getByRole('combobox', { name: 'Device' })).toHaveValue('demo-1')
    expect(screen.getByRole('button', { name: 'Format' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Refresh demo data' }))
    await waitFor(() =>
      expect(screen.getByRole('combobox', { name: 'Device' })).toHaveValue('demo-1'),
    )
    expect(screen.getByText('Demonstration data reloaded · no hardware scan')).toBeVisible()
    await user.click(nav('Disk Overview'))
    expect(screen.getByRole('button', { name: 'Select Archive' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })
  it('USB targets exclude internal disks and never enable writes', async () => {
    const user = userEvent.setup()
    render(<App />)
    await screen.findByRole('region', { name: 'Selected device' })
    await user.click(nav('USB Writer'))
    expect(screen.getByRole('option', { name: /Portable USB/ })).toBeInTheDocument()
    expect(screen.queryByRole('option', { name: /System Drive/ })).not.toBeInTheDocument()
    await user.selectOptions(screen.getByLabelText('Demonstration target'), 'demo-2')
    expect(screen.getByRole('button', { name: 'Write image' })).toBeDisabled()
    expect(screen.getByRole('button', { name: /Choose image/ })).toBeDisabled()
    await user.click(nav('Disk Overview'))
    expect(screen.getByRole('button', { name: 'Select Portable USB' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })
  it('handles native navigation and refresh, and unsubscribes on unmount', async () => {
    const { unmount } = render(<App />)
    await screen.findByRole('region', { name: 'Selected device' })
    act(() => menuAction('settings'))
    expect(screen.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible()
    act(() => menuAction('refresh'))
    await waitFor(() => expect(devices.loadDemoDisks).toHaveBeenCalledTimes(2))
    unmount()
    expect(stopListening).toHaveBeenCalledTimes(1)
  })
  it('persists a theme and keeps browser preview honest', async () => {
    const user = userEvent.setup()
    render(<App />)
    await user.click(nav('Settings'))
    await user.click(screen.getByRole('button', { name: 'Dark' }))
    expect(document.documentElement).toHaveClass('dark')
    expect(localStorage.getItem('diskvio-theme')).toBe('dark')
    expect(await screen.findByText(/Browser preview. Native application/)).toBeVisible()
    expect(screen.queryByText('Connected through Tauri IPC')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Light' }))
    expect(document.documentElement).not.toHaveClass('dark')
  })
  it('renders loading, empty and recoverable error states', async () => {
    vi.mocked(devices.loadDemoDisks).mockResolvedValueOnce([])
    const user = userEvent.setup()
    render(<App />)
    expect(screen.getByRole('heading', { name: 'Loading demonstration devices' })).toBeVisible()
    expect(await screen.findByRole('heading', { name: 'No demonstration devices' })).toBeVisible()
    vi.mocked(devices.loadDemoDisks).mockRejectedValueOnce(new Error('Fixture unavailable'))
    await user.click(screen.getByRole('button', { name: 'Try again' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Fixture unavailable')
    await user.click(screen.getByRole('button', { name: 'Try again' }))
    expect(await screen.findByRole('region', { name: 'Selected device' })).toBeVisible()
  })
  it('reports native IPC failure and supports retry', async () => {
    vi.mocked(native.getAppInfo).mockRejectedValueOnce(new Error('IPC disconnected'))
    const user = userEvent.setup()
    render(<App />)
    await user.click(nav('Settings'))
    expect(await screen.findByRole('alert')).toHaveTextContent('IPC disconnected')
    vi.mocked(native.getAppInfo).mockResolvedValueOnce({
      name: 'Diskvio',
      version: '2.0.0',
      platform: 'windows',
      dataMode: 'demo',
      capabilities: { discovery: false, partitionManagement: false, usbWriting: false },
    })
    await user.click(screen.getByRole('button', { name: 'Retry connection' }))
    expect(await screen.findByText('Connected through Tauri IPC')).toBeVisible()
  })
})
