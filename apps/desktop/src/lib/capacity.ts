import type { Bytes, Disk, Partition, Segment } from '@/models/disk'

export function formatCapacity(bytes: Bytes | bigint | number | null): string {
  if (bytes === null) return 'Unknown'
  if (typeof bytes === 'number' && !Number.isSafeInteger(bytes)) throw new Error('Unsafe capacity')
  const n = BigInt(bytes)
  if (n < 0n) throw new Error('Capacity must be non-negative')
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB']
  let divisor = 1n,
    unit = 0
  while (n >= divisor * 1000n && unit < units.length - 1) {
    divisor *= 1000n
    unit++
  }
  if (!unit) return `${n} B`
  const tenths = (n * 10n + divisor / 2n) / divisor
  return `${tenths / 10n}.${tenths % 10n} ${units[unit]}`
}
export const partitionName = (p: Partition) =>
  p.volumes
    .map((v) => v.label)
    .filter(Boolean)
    .join(', ') ||
  p.name ||
  p.partitionType ||
  `Partition ${p.number ?? '?'}`
export const allocatedCapacity = (d: Disk) =>
  d.partitions === null ? null : d.partitions.reduce((n, p) => n + BigInt(p.capacityBytes), 0n)
// Shared/unknown filesystems cannot be summed into exclusive physical usage.
function usage(d: Disk, key: 'usedBytes' | 'freeBytes'): bigint | null {
  if (d.partitions === null) return null
  let sum = 0n
  for (const p of d.partitions) {
    if (p.volumes.length !== 1) return null
    const v = p.volumes[0]
    if (v.sharedSpace || v[key] === null) return null
    sum += BigInt(v[key])
  }
  return sum
}
export const usedCapacity = (d: Disk) => usage(d, 'usedBytes')
export const freeCapacity = (d: Disk) => usage(d, 'freeBytes')
export const unallocatedCapacity = (d: Disk) => {
  const n = allocatedCapacity(d)
  return n === null ? null : BigInt(d.capacityBytes) - n
}
export const ratio = (n: bigint, total: string | bigint) => (Number(n) / Number(total)) * 100

/** Geometry uses bigint; only final visual proportions convert to floating point. */
export function partitionSegments(disk: Disk): Segment[] {
  if (disk.partitions === null) return []
  const segments: Segment[] = []
  let cursor = 0n
  const addGap = (end: bigint) => {
    if (end > cursor)
      segments.push({
        id: `gap-${cursor}`,
        name: 'Unallocated',
        capacityBytes: end - cursor,
        percent: ratio(end - cursor, disk.capacityBytes),
        partition: null,
        colorIndex: -1,
      })
  }
  disk.partitions.forEach((p, index) => {
    addGap(BigInt(p.offsetBytes))
    segments.push({
      id: p.id,
      name: partitionName(p),
      capacityBytes: BigInt(p.capacityBytes),
      percent: ratio(BigInt(p.capacityBytes), disk.capacityBytes),
      partition: p,
      colorIndex: index % 4,
    })
    cursor = BigInt(p.offsetBytes) + BigInt(p.capacityBytes)
  })
  addGap(BigInt(disk.capacityBytes))
  return segments
}
