import type { Disk } from '@/models/disk'
import { formatCapacity, allocatedCapacity } from '@/lib/capacity'
export function PartitionTable({ disk }: { disk: Disk }) {
  const unallocated = disk.capacityBytes - allocatedCapacity(disk)
  return (
    <div
      className="table-scroll"
      role="region"
      aria-label={`Partition table for ${disk.name}`}
      tabIndex={0}
    >
      <table className="partition-table">
        <caption className="sr-only">Partitions on {disk.name}, sample data</caption>
        <thead>
          <tr>
            <th scope="col">Partition</th>
            <th scope="col">File system</th>
            <th scope="col">Location</th>
            <th scope="col" className="numeric">
              Size
            </th>
            <th scope="col" className="numeric">
              Used
            </th>
          </tr>
        </thead>
        <tbody>
          {disk.partitions.map((p, index) => (
            <tr key={p.id}>
              <th scope="row" title={p.name}>
                <i className={`table-swatch partition-color-${index % 4}`} />
                {p.name === 'EFI System' ? 'Startup files' : p.name}
              </th>
              <td>{p.filesystem ?? 'None'}</td>
              <td>
                <code>{p.mountPoint ?? 'None'}</code>
              </td>
              <td className="numeric">{formatCapacity(p.capacityBytes)}</td>
              <td className="numeric">{formatCapacity(p.usedBytes)}</td>
            </tr>
          ))}
          {unallocated > 0 && (
            <tr>
              <th scope="row">
                <i className="table-swatch partition-color--1" />
                Unassigned space
              </th>
              <td>None</td>
              <td>None</td>
              <td className="numeric">{formatCapacity(unallocated)}</td>
              <td className="numeric">0 B</td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
