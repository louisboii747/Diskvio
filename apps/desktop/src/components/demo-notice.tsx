import { FlaskConical } from 'lucide-react'
export function DemoNotice() {
  return (
    <div className="demo-notice">
      <FlaskConical size={16} aria-hidden="true" />
      <strong>DEMO DATA</strong>
      <span>These are sample drives. Your files stay untouched.</span>
    </div>
  )
}
