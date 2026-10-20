import { useCallback, useEffect, useRef, useState } from 'react'
import type { Disk, InventorySource } from '@/models/disk'
import { loadDemoDisks } from '@/services/devices'
import { isNative, listDisks } from '@/services/native'

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return String(error)
}
export function sameDevice(a: Disk, b: Disk) {
  return (
    a.id === b.id &&
    a.serial === b.serial &&
    a.model === b.model &&
    a.capacityBytes === b.capacityBytes
  )
}
export function eligibleUsb(d: Disk) {
  return (
    d.connection?.split(/[ /]/)[0].toUpperCase() === 'USB' &&
    (d.kind === 'external' || d.kind === 'removable') &&
    d.system === false &&
    d.online !== false &&
    d.readOnly !== true
  )
}
function initialSource(): InventorySource {
  if (!isNative()) return 'demo'
  try {
    return localStorage.getItem('diskvio-source') === 'demo' ? 'demo' : 'live'
  } catch {
    return 'live'
  }
}
export function useInventory() {
  const [source, setSource] = useState<InventorySource>(initialSource)
  const [state, setState] = useState<{
    disks: Disk[]
    selected: Disk | null
    loading: boolean
    error: string | null
    notice: string | null
  }>({ disks: [], selected: null, loading: true, error: null, notice: null })
  const generation = useRef(0)
  const refresh = useCallback(async () => {
    const request = ++generation.current
    setState((s) => ({ ...s, loading: true, error: null, notice: null }))
    try {
      const disks = await (source === 'live' ? listDisks() : loadDemoDisks())
      if (request !== generation.current) return
      setState((s) => {
        const found = s.selected && disks.find((d) => sameDevice(s.selected!, d))
        const lost = s.selected && !found
        return {
          disks,
          selected: found || (lost ? null : (disks[0] ?? null)),
          loading: false,
          error: null,
          notice: lost
            ? 'The selected drive was removed or its identity changed. Choose a drive.'
            : null,
        }
      })
    } catch (e) {
      if (request === generation.current)
        setState((s) => ({ ...s, loading: false, error: errorMessage(e) }))
    }
  }, [source])
  useEffect(() => {
    const sequence = generation
    // The initial state already represents loading; refresh also handles menu requests.
    void refresh()
    return () => {
      sequence.current++
    }
  }, [refresh])
  const changeSource = (next: InventorySource) => {
    if (next === source || (next === 'live' && !isNative())) return
    generation.current++
    setState({ disks: [], selected: null, loading: true, error: null, notice: null })
    setSource(next)
    if (isNative()) {
      try {
        localStorage.setItem('diskvio-source', next)
      } catch {
        /* Preferences are optional. */
      }
    }
  }
  const select = (id: string) =>
    setState((s) => ({ ...s, selected: s.disks.find((d) => d.id === id) ?? null, notice: null }))
  return { ...state, source, changeSource, refresh, select }
}
