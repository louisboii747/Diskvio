import { LockKeyhole, Plus, Trash2, Scaling, Eraser } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { Disk } from '@/models/disk'
import { PartitionMap } from '@/features/disks/partition-map'
import { PartitionTable } from '@/features/disks/partition-table'
export function PartitionsScreen({
  disks,
  selected,
  select,
}: {
  disks: Disk[]
  selected: Disk
  select: (id: string) => void
}) {
  return (
    <div className="feature-stack">
      <section className="panel">
        <div className="section-title">
          <h2>Partition inspector</h2>
          <label className="device-picker">
            Device
            <select value={selected.id} onChange={(event) => select(event.target.value)}>
              {disks.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="partition-intro">
          <strong>{selected.model}</strong>
          <span>
            {selected.partitionScheme} · {selected.partitions.length} partitions · simulated device
          </span>
        </div>
        <PartitionMap disk={selected} />
        <PartitionTable disk={selected} />
      </section>
      <section className="panel planned-actions">
        <div>
          <LockKeyhole size={20} />
          <h2>Partition operations</h2>
          <span className="scheme-tag">Not Yet Available</span>
        </div>
        <p>
          Future operations will require capability checks, target identity revalidation and
          explicit confirmation. No operation can be performed in this milestone.
        </p>
        <div className="button-row">
          <Button disabled variant="outline">
            <Plus />
            Create partition
          </Button>
          <Button disabled variant="outline">
            <Eraser />
            Format
          </Button>
          <Button disabled variant="outline">
            <Scaling />
            Resize
          </Button>
          <Button disabled variant="outline">
            <Trash2 />
            Delete
          </Button>
        </div>
      </section>
    </div>
  )
}
