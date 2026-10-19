import { FlaskConical } from 'lucide-react'
export function DemoNotice() {
  return (
    <div className="demo-notice">
      <FlaskConical size={17} aria-hidden="true" />
      <div>
        <strong>DEMO DATA / SIMULATED DEVICES</strong>
        <span>Explore sample layouts. Your physical disks are not accessed or modified.</span>
      </div>
    </div>
  )
}
