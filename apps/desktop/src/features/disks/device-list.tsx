import { HardDrive, Usb, ChevronRight } from 'lucide-react'
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
    <section className="device-list" aria-label="Demonstration devices">
      <div className="panel-heading">
        <h2>Devices</h2>
        <span>{disks.length} simulated</span>
      </div>
      <div className="device-buttons">
        {disks.map((disk, index) => {
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
                <Icon size={20} />
                <span>DISK {index.toString().padStart(2, '0')}</span>
                <ChevronRight size={14} />
              </div>
              <strong>{disk.name}</strong>
              <span className="device-model">{disk.model}</span>
              <div className="device-meta">
                <span>{disk.kind === 'removable' ? 'Removable' : 'Internal'}</span>
                <strong>{formatCapacity(disk.capacityBytes)}</strong>
              </div>
              <div className="usage-track" aria-hidden="true">
                <span style={{ width: `${(used / disk.capacityBytes) * 100}%` }} />
              </div>
              <span className="device-usage">
                {formatCapacity(used)} used · {Math.round((used / disk.capacityBytes) * 100)}%
              </span>
            </button>
          )
        })}
      </div>
      <p className="panel-footnote">Only sample devices are listed.</p>
    </section>
  )
}
