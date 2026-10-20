/** Rust u64 values cross IPC as decimal strings. Arithmetic uses bigint. */
export type Bytes = string
export interface Volume {
  id: string
  label: string | null
  filesystem: string | null
  mountPoints: string[]
  capacityBytes: Bytes | null
  usedBytes: Bytes | null
  freeBytes: Bytes | null
  sharedSpace: boolean
}
export interface Partition {
  id: string
  number: number | null
  name: string | null
  partitionType: string | null
  offsetBytes: Bytes
  capacityBytes: Bytes
  volumes: Volume[]
}
export interface Disk {
  id: string
  devicePath: string
  name: string
  manufacturer: string | null
  model: string | null
  serial: string | null
  kind: 'internal' | 'external' | 'removable' | 'unknown'
  removable: boolean | null
  connection: string | null
  partitionScheme: string | null
  online: boolean | null
  readOnly: boolean | null
  system: boolean | null
  capacityBytes: Bytes
  partitions: Partition[] | null
  limitations: string[]
}
export interface Segment {
  id: string
  name: string
  capacityBytes: bigint
  percent: number
  partition: Partition | null
  colorIndex: number
}
export type InventorySource = 'live' | 'demo'
