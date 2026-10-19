import { ArrowRight, HardDrive } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { formatCapacity, usedCapacity, allocatedCapacity } from '@/lib/capacity'
import { DeviceList } from './device-list'
import { PartitionMap } from './partition-map'
import { PartitionTable } from './partition-table'
import type { Disk } from '@/models/disk'

export function DiskOverview({
  disks,
  selected,
  select,
  openPartitions,
}: {
  disks: Disk[]
  selected: Disk
  select: (id: string) => void
  openPartitions: () => void
}) {
  const used = usedCapacity(selected)
  const allocated = allocatedCapacity(selected)
  return (
    <div className="disk-workspace">
      <DeviceList disks={disks} selected={selected.id} select={select} />
      <section className="panel disk-detail" aria-label="Selected device">
        <div className="selected-heading">
          <div className="disk-icon">
            <HardDrive size={23} />
          </div>
          <div>
            <h2>{selected.name}</h2>
            <p>{selected.model}</p>
          </div>
          <Button variant="outline" size="sm" onClick={openPartitions}>
            View partitions
            <ArrowRight size={14} />
          </Button>
        </div>
        <div className="capacity-row">
          <div>
            <span>Total space</span>
            <strong>{formatCapacity(selected.capacityBytes)}</strong>
          </div>
          <div>
            <span>Used</span>
            <strong>{formatCapacity(used)}</strong>
          </div>
          <div title="Space for more files in existing partitions">
            <span>Free for files</span>
            <strong>{formatCapacity(allocated - used)}</strong>
          </div>
          <div title="Space that does not belong to a partition">
            <span>Unassigned</span>
            <strong>{formatCapacity(selected.capacityBytes - allocated)}</strong>
          </div>
        </div>
        <PartitionMap disk={selected} />
        <PartitionTable disk={selected} />
        <dl className="device-facts">
          <div>
            <dt>Connection</dt>
            <dd>{selected.connection}</dd>
          </div>
          <div>
            <dt>Layout type</dt>
            <dd>{selected.partitionScheme}</dd>
          </div>
          <div>
            <dt>Drive ID</dt>
            <dd>
              <code>{selected.id}</code>
            </dd>
          </div>
          <div>
            <dt>Serial number</dt>
            <dd>
              <code>{selected.serial}</code>
            </dd>
          </div>
        </dl>
      </section>
    </div>
  )
}
