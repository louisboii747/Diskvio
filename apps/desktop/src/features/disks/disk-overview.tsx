import { ArrowRight, HardDrive, LockKeyhole } from 'lucide-react'
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
    <>
      <div className="overview-summary">
        <span>
          <strong>{disks.length}</strong> physical disks
        </span>
        <span>
          <strong>{formatCapacity(disks.reduce((n, d) => n + d.capacityBytes, 0))}</strong> total
          capacity
        </span>
        <span>
          <strong>{disks.filter((d) => d.kind === 'removable').length}</strong> removable device
        </span>
        <span className="muted">Sample inventory</span>
      </div>
      <div className="disk-workspace">
        <DeviceList disks={disks} selected={selected.id} select={select} />
        <div className="disk-detail">
          <section className="panel" aria-label="Selected device">
            <div className="selected-heading">
              <div className="disk-icon">
                <HardDrive size={25} />
              </div>
              <div>
                <div className="eyebrow">
                  {selected.kind === 'removable' ? 'REMOVABLE DEVICE' : 'INTERNAL DEVICE'}
                </div>
                <h2>{selected.name}</h2>
                <p>{selected.model}</p>
              </div>
              <span className="scheme-tag">{selected.partitionScheme}</span>
            </div>
            <div className="capacity-row">
              <div>
                <span>Total capacity</span>
                <strong>{formatCapacity(selected.capacityBytes)}</strong>
              </div>
              <div>
                <span>Used</span>
                <strong>{formatCapacity(used)}</strong>
              </div>
              <div>
                <span>Free in partitions</span>
                <strong>{formatCapacity(allocated - used)}</strong>
              </div>
              <div>
                <span>Unallocated</span>
                <strong>{formatCapacity(selected.capacityBytes - allocated)}</strong>
              </div>
            </div>
            <div className="section-title">
              <h3>Partition layout</h3>
              <span>{selected.partitions.length} partitions</span>
            </div>
            <PartitionMap disk={selected} />
            <PartitionTable disk={selected} />
          </section>
          <section className="panel device-details">
            <div className="section-title">
              <h3>Device information</h3>
              <span>SIMULATED</span>
            </div>
            <dl>
              <div>
                <dt>Connection</dt>
                <dd>{selected.connection}</dd>
              </div>
              <div>
                <dt>Device identifier</dt>
                <dd>
                  <code>{selected.id}</code>
                </dd>
              </div>
              <div>
                <dt>Partition scheme</dt>
                <dd>{selected.partitionScheme}</dd>
              </div>
              <div>
                <dt>Serial number</dt>
                <dd>
                  <code>{selected.serial}</code>
                </dd>
              </div>
            </dl>
          </section>
          <section className="action-panel">
            <LockKeyhole size={18} />
            <div>
              <strong>Inspection comes first.</strong>
              <p>Formatting, resizing and ejecting are not yet available.</p>
            </div>
            <Button variant="outline" size="sm" onClick={openPartitions}>
              Inspect partitions
              <ArrowRight size={14} />
            </Button>
          </section>
        </div>
      </div>
    </>
  )
}
