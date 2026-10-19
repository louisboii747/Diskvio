export interface Partition {
  id: string
  name: string
  offsetBytes: number
  capacityBytes: number
  usedBytes: number
  filesystem: string | null
  mountPoint: string | null
}
export interface Disk {
  id: string
  name: string
  model: string
  serial: string
  kind: 'internal' | 'removable'
  connection: string
  partitionScheme: string
  capacityBytes: number
  partitions: Partition[]
}
export interface Segment {
  id: string
  name: string
  capacityBytes: number
  percent: number
  partition: Partition | null
  colorIndex: number
}
