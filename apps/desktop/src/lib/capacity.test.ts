import { describe, it, expect } from 'vitest'
import { formatCapacity, partitionSegments } from './capacity'
import fixtures from '@/fixtures/disks.json'
import type { Disk } from '@/models/disk'
const disks = fixtures as Disk[]
describe('capacity and proportional layouts', () => {
  it('uses decimal capacity units consistently', () => {
    expect(formatCapacity(0)).toBe('0 B')
    expect(formatCapacity(1000)).toBe('1.0 KB')
    expect(formatCapacity(64000000000)).toBe('64.0 GB')
    expect(() => formatCapacity(-1)).toThrow()
    expect(() => formatCapacity(Infinity)).toThrow()
  })
  it('preserves small system partitions without a visual minimum', () => {
    expect(partitionSegments(disks[0]).map((p) => p.percent)).toEqual([0.1, 74.9, 24.9, 0.1])
  })
  it('includes the exact unallocated capacity', () => {
    const segments = partitionSegments(disks[1])
    expect(segments.map((p) => p.percent)).toEqual([90, 10])
    expect(segments[1].name).toBe('Unallocated')
    expect(segments[1].capacityBytes).toBe(200000000000)
  })
  it('accounts for leading gaps and empty layouts', () => {
    const disk = {
      ...disks[2],
      partitions: [
        { ...disks[2].partitions[0], offsetBytes: 1000000000, capacityBytes: 63000000000 },
      ],
    }
    expect(partitionSegments(disk)[0].capacityBytes).toBe(1000000000)
    expect(partitionSegments({ ...disk, partitions: [] })[0].percent).toBe(100)
  })
  it('every fixture fills 100 percent including gaps', () => {
    for (const disk of disks)
      expect(partitionSegments(disk).reduce((n, p) => n + p.percent, 0)).toBeCloseTo(100)
  })
})
