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
    <section className="panel partitions-panel">
      <div className="section-title">
        <div>
          <h2>{selected.name}</h2>
          <p>
            {selected.model ?? 'Model not available'} ·{' '}
            {selected.partitionScheme ?? 'Unknown layout'}
          </p>
        </div>
        <label className="device-picker">
          Drive
          <select value={selected.id} onChange={(event) => select(event.target.value)}>
            {disks.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </select>
        </label>
      </div>
      <PartitionMap disk={selected} />
      <PartitionTable disk={selected} />
      <p className="table-help">
        The file system controls how a partition stores files. Location shows where you can open it.
      </p>
      <div className="planned-actions">
        <div className="availability-note">
          <LockKeyhole size={16} />
          <span>Changing partitions isn't available yet.</span>
        </div>
        <div className="button-row">
          <Button disabled variant="outline">
            <Plus />
            Create
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
      </div>
    </section>
  )
}
