import { HardDrive, Usb, Check } from 'lucide-react'
import { cn } from '@/lib/utils'
import { formatCapacity, usedCapacity, ratio } from '@/lib/capacity'
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
    <section className="device-list" aria-label="Drives">
      <div className="panel-heading">
        <h2>Drives</h2>
        <span>{disks.length} devices</span>
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
                <span>
                  {disk.kind === 'removable'
                    ? 'Removable drive'
                    : disk.kind === 'external'
                      ? 'External drive'
                      : disk.kind === 'internal'
                        ? 'Built-in drive'
                        : 'Unknown location'}
                </span>
                {selected === disk.id && <Check size={14} />}
              </div>
              <strong>{disk.name}</strong>
              <span className="device-model">{disk.model ?? 'Model not available'}</span>
              <div className="device-meta">
                <strong>{formatCapacity(disk.capacityBytes)}</strong>
                <span>
                  {used === null
                    ? 'Usage unknown'
                    : `${Math.round(ratio(used, disk.capacityBytes))}% used`}
                </span>
              </div>
              <div className="usage-track" aria-hidden="true">
                <span
                  style={{ width: `${used === null ? 0 : ratio(used, disk.capacityBytes)}%` }}
                />
              </div>
            </button>
          )
        })}
      </div>
    </section>
  )
}
