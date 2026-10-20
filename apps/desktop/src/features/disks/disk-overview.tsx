import { ArrowRight, HardDrive } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { formatCapacity, usedCapacity, freeCapacity, unallocatedCapacity } from '@/lib/capacity'
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
            <p>{selected.model ?? 'Model not available'}</p>
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
            <strong>{formatCapacity(freeCapacity(selected))}</strong>
          </div>
          <div title="Space that does not belong to a partition">
            <span>Unallocated</span>
            <strong>{formatCapacity(unallocatedCapacity(selected))}</strong>
          </div>
        </div>
        <PartitionMap disk={selected} />
        <PartitionTable disk={selected} />
        <dl className="device-facts">
          <div>
            <dt>Connection</dt>
            <dd>
              {selected.connection ?? 'Unknown'}
              {selected.online === false ? ' · Offline' : ''}
              {selected.readOnly === true ? ' · Read only' : ''}
            </dd>
          </div>
          <div>
            <dt>Layout type</dt>
            <dd>{selected.partitionScheme ?? 'Unknown'}</dd>
          </div>
          <div>
            <dt>Drive ID</dt>
            <dd>
              <code title={selected.devicePath}>{selected.devicePath}</code>
            </dd>
          </div>
          <div>
            <dt>Serial number</dt>
            <dd>
              <code>{selected.serial ?? 'Not available'}</code>
            </dd>
          </div>
        </dl>
        {selected.limitations.length > 0 && (
          <p className="table-help">{selected.limitations.join(' ')}</p>
        )}
      </section>
    </div>
  )
}
