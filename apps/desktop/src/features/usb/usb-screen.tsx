import { FileImage, Usb, ShieldCheck, LockKeyhole } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { formatCapacity } from '@/lib/capacity'
import type { Disk } from '@/models/disk'
export function UsbScreen({
  disks,
  selectedId,
  select,
}: {
  disks: Disk[]
  selectedId: string
  select: (id: string) => void
}) {
  const removable = disks.filter((d) => d.kind === 'removable')
  return (
    <section className="panel usb-panel">
      <div className="section-title">
        <h2>Make a bootable USB drive</h2>
        <span className="scheme-tag">Coming later</span>
      </div>
      <p className="intro-copy">
        Use a USB drive to install an operating system, such as Windows or Linux.
      </p>
      <div className="usb-steps">
        <section className="usb-step">
          <div className="step-heading">
            <span className="step-number">1</span>
            <h3>Choose a file</h3>
          </div>
          <div className="step-body">
            <FileImage size={32} />
            <strong>No file chosen</strong>
            <p>You'll need an ISO or IMG file for the system you want to install.</p>
          </div>
          <Button disabled variant="outline">
            Choose file
          </Button>
        </section>
        <section className="usb-step">
          <div className="step-heading">
            <span className="step-number">2</span>
            <h3>Choose a USB drive</h3>
          </div>
          <div className="step-body">
            <Usb size={32} />
            <label htmlFor="usb-target">Sample USB drive</label>
            <select
              id="usb-target"
              value={removable.some((d) => d.id === selectedId) ? selectedId : ''}
              onChange={(event) => select(event.target.value)}
            >
              <option value="" disabled>
                Choose a drive
              </option>
              {removable.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name} · {formatCapacity(d.capacityBytes)}
                </option>
              ))}
            </select>
            <p>
              {removable.length
                ? 'You can try choosing a sample drive.'
                : 'No sample USB drives to show.'}
            </p>
          </div>
          <span className="step-caption">Sample drives only</span>
        </section>
        <section className="usb-step">
          <div className="step-heading">
            <span className="step-number">3</span>
            <h3>Write to USB</h3>
          </div>
          <div className="step-body">
            <ShieldCheck size={32} />
            <strong>Check before writing</strong>
            <p>
              Writing will replace all files on the USB drive. You'll need to confirm this first.
            </p>
          </div>
          <Button disabled>Write to USB</Button>
        </section>
      </div>
      <div className="usb-footer availability-note">
        <LockKeyhole size={16} />
        <span>Choosing files and writing to USB aren't available yet.</span>
      </div>
    </section>
  )
}
