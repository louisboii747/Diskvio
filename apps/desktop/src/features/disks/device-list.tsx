import { HardDrive, Usb, Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import { formatCapacity, usedCapacity } from '@/lib/capacity'
import type { Disk } from '@/models/disk'
export function DeviceList({
  disks,
  selected,
  select,
}: {
  disks: Disk[]
  selected: string
  select: (id: string) => void
}) {
  return (
    <section className="device-list" aria-label="Sample drives">
      <div className="panel-heading">
        <h2>Drives</h2>
        <span>{disks.length} samples</span>
      </div>
      <div className="device-buttons">
        {disks.map((disk) => {
          const Icon = disk.kind === 'removable' ? Usb : HardDrive
          const used = usedCapacity(disk)
          return (
            <button
              key={disk.id}
              className={cn('device-button', selected === disk.id && 'device-selected')}
              aria-label={`Select ${disk.name}`}
              aria-pressed={selected === disk.id}
              onClick={() => select(disk.id)}
            >
              <div className="device-top">
                <Icon size={18} />
                <span>{disk.kind === 'removable' ? 'USB drive' : 'Built-in drive'}</span>
                {selected === disk.id && <Check size={14} />}
              </div>
              <strong>{disk.name}</strong>
              <span className="device-model">{disk.model}</span>
              <div className="device-meta">
                <strong>{formatCapacity(disk.capacityBytes)}</strong>
                <span>{Math.round((used / disk.capacityBytes) * 100)}% used</span>
              </div>
              <div className="usage-track" aria-hidden="true">
                <span style={{ width: `${(used / disk.capacityBytes) * 100}%` }} />
              </div>
            </button>
          )
        })}
      </div>
    </section>
  )
}
