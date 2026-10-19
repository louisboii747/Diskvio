import type { Disk, Segment } from '@/models/disk'

export function formatCapacity(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0)
    throw new Error('Capacity must be non-negative and finite.')
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB']
  const unit = bytes === 0 ? 0 : Math.min(Math.floor(Math.log10(bytes) / 3), units.length - 1)
  return unit === 0 ? `${bytes} B` : `${(bytes / 1000 ** unit).toFixed(1)} ${units[unit]}`
}
export const usedCapacity = (disk: Disk) => disk.partitions.reduce((sum, p) => sum + p.usedBytes, 0)
export const allocatedCapacity = (disk: Disk) =>
  disk.partitions.reduce((sum, p) => sum + p.capacityBytes, 0)

/** Preserve exact proportions, including gaps. Never impose visual minimum widths. */
export function partitionSegments(disk: Disk): Segment[] {
  const segments: Segment[] = []
  let cursor = 0
  disk.partitions.forEach((partition, index) => {
    if (partition.offsetBytes > cursor)
      segments.push({
        id: `gap-${cursor}`,
        name: 'Unallocated',
        capacityBytes: partition.offsetBytes - cursor,
        percent: ((partition.offsetBytes - cursor) / disk.capacityBytes) * 100,
        partition: null,
        colorIndex: -1,
      })
    segments.push({
      id: partition.id,
      name: partition.name,
      capacityBytes: partition.capacityBytes,
      percent: (partition.capacityBytes / disk.capacityBytes) * 100,
      partition,
      colorIndex: index % 4,
    })
    cursor = partition.offsetBytes + partition.capacityBytes
  })
  if (cursor < disk.capacityBytes)
    segments.push({
      id: `gap-${cursor}`,
      name: 'Unallocated',
      capacityBytes: disk.capacityBytes - cursor,
      percent: ((disk.capacityBytes - cursor) / disk.capacityBytes) * 100,
      partition: null,
      colorIndex: -1,
    })
  return segments
}
