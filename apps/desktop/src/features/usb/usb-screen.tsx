import { FileImage, Usb, ShieldCheck, ArrowRight } from 'lucide-react'
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
    <div className="usb-layout">
      <section className="panel usb-workflow">
        <div className="section-title">
          <h2>Write a bootable USB</h2>
          <span className="scheme-tag">Not Yet Available</span>
        </div>
        <p className="intro-copy">
          A preview of the planned workflow. Image selection and writing are unavailable in
          Milestone 1.
        </p>
        <div className="workflow-step">
          <span className="step-number">01</span>
          <div>
            <h3>Choose an image</h3>
            <p>Select an operating system image to write.</p>
          </div>
        </div>
        <div className="image-placeholder">
          <FileImage size={30} />
          <strong>No image selected</strong>
          <span>ISO / IMG support is planned</span>
          <Button disabled variant="outline" size="sm">
            Choose image — Not Yet Available
          </Button>
        </div>
        <div className="workflow-step">
          <span className="step-number">02</span>
          <div>
            <h3>Review the target</h3>
            <p>Only simulated removable devices are shown.</p>
          </div>
        </div>
        <label className="usb-target-label" htmlFor="usb-target">
          Demonstration target
        </label>
        <select
          id="usb-target"
          value={removable.some((d) => d.id === selectedId) ? selectedId : ''}
          onChange={(event) => select(event.target.value)}
        >
          <option value="" disabled>
            Select a simulated USB device
          </option>
          {removable.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name} · {formatCapacity(d.capacityBytes)} · {d.serial}
            </option>
          ))}
        </select>
        {removable.length === 0 && (
          <p role="status">No removable demonstration devices are available.</p>
        )}
        <div className="workflow-step">
          <span className="step-number">03</span>
          <div>
            <h3>Confirm and write</h3>
            <p>Future writes will erase the selected target.</p>
          </div>
        </div>
        <Button disabled className="write-button">
          <Usb size={16} />
          Write image
          <ArrowRight size={16} />
        </Button>
        <p className="map-note">Not Yet Available. No files are opened and no data is written.</p>
      </section>
      <aside className="panel usb-guidance">
        <ShieldCheck size={25} />
        <h2>A deliberate process</h2>
        <p>
          Writing an image replaces data on the target device. The future writer must make every
          consequence clear before it begins.
        </p>
        <ul>
          <li>Identify the exact physical device</li>
          <li>Recheck its identity before writing</li>
          <li>Require an explicit erase confirmation</li>
          <li>Report verification and failures honestly</li>
        </ul>
        <div className="guidance-note">
          These safeguards describe the planned implementation. Writing remains disabled.
        </div>
      </aside>
    </div>
  )
}
