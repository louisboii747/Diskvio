import { describe, it, expect } from 'vitest'
import {
  formatCapacity,
  partitionSegments,
  usedCapacity,
  freeCapacity,
  unallocatedCapacity,
} from './capacity'
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
    expect(segments[1].capacityBytes).toBe(200000000000n)
  })
  it('accounts for leading gaps and empty layouts', () => {
    const disk = {
      ...disks[2],
      partitions: [
        { ...disks[2].partitions![0], offsetBytes: '1000000000', capacityBytes: '63000000000' },
      ],
    }
    expect(partitionSegments(disk)[0].capacityBytes).toBe(1000000000n)
    expect(partitionSegments({ ...disk, partitions: [] })[0].percent).toBe(100)
  })
  it('every fixture fills 100 percent including gaps', () => {
    for (const disk of disks)
      expect(partitionSegments(disk).reduce((n, p) => n + p.percent, 0)).toBeCloseTo(100)
  })
  it('keeps unknown usage and inaccessible layouts distinct from zero', () => {
    const d = { ...disks[0], partitions: null }
    expect(usedCapacity(d)).toBeNull()
    expect(freeCapacity(d)).toBeNull()
    expect(unallocatedCapacity(d)).toBeNull()
    expect(partitionSegments(d)).toEqual([])
    const p = disks[0].partitions![0]
    const unknown = {
      ...disks[0],
      partitions: [{ ...p, volumes: [{ ...p.volumes[0], usedBytes: null }] }],
    }
    expect(usedCapacity(unknown)).toBeNull()
    expect(formatCapacity(null)).toBe('Unknown')
    expect(
      usedCapacity({
        ...disks[0],
        partitions: [{ ...p, volumes: [{ ...p.volumes[0], sharedSpace: true }] }],
      }),
    ).toBeNull()
  })
  it('does exact arithmetic above the JavaScript safe integer boundary', () => {
    const d = {
      ...disks[0],
      capacityBytes: '18446744073709551615',
      partitions: [
        { ...disks[0].partitions![0], offsetBytes: '9007199254740993', capacityBytes: '1' },
      ],
    }
    expect(partitionSegments(d)[0].capacityBytes).toBe(9007199254740993n)
    expect(unallocatedCapacity(d)).toBe(18446744073709551614n)
    expect(formatCapacity(d.capacityBytes)).toBe('18.4 EB')
    expect(() => formatCapacity(Number.MAX_SAFE_INTEGER + 1)).toThrow()
  })
})
