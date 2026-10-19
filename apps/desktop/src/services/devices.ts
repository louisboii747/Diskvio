import fixtures from '@/fixtures/disks.json'
import type { Disk } from '@/models/disk'

/** Local demonstration fixtures only. This service never discovers real devices. */
export async function loadDemoDisks(): Promise<Disk[]> {
  return structuredClone(fixtures) as Disk[]
}
