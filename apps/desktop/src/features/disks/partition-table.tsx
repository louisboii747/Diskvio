import type { Disk } from '@/models/disk'
import { formatCapacity, partitionSegments, partitionName } from '@/lib/capacity'
export function PartitionTable({ disk }: { disk: Disk }) {
  if (disk.partitions === null) return null
  return (
    <div
      className="table-scroll"
      role="region"
      aria-label={`Partition table for ${disk.name}`}
      tabIndex={0}
    >
      <table className="partition-table">
        <caption className="sr-only">Partitions on {disk.name}</caption>
        <thead>
          <tr>
            <th scope="col">Partition</th>
            <th scope="col">File system</th>
            <th scope="col">Location</th>
            <th scope="col" className="numeric">
              Size
            </th>
            <th scope="col" className="numeric">
              Used / free
            </th>
          </tr>
        </thead>
        <tbody>
          {partitionSegments(disk).map((s) => {
            const p = s.partition
            return (
              <tr key={s.id}>
                <th
                  scope="row"
                  title={
                    p
                      ? `#${p.number ?? '?'} · ${p.partitionType ?? 'Unknown type'} · Offset ${p.offsetBytes} bytes · Length ${p.capacityBytes} bytes`
                      : 'Physical space outside partitions'
                  }
                >
                  <i className={`table-swatch partition-color-${s.colorIndex}`} />
                  {p ? `${p.number ? `#${p.number} ` : ''}${partitionName(p)}` : 'Unassigned space'}
                  {p && p.volumes.length > 1 && <small> · Shared volumes</small>}
                </th>
                <td>
                  {p
                    ? p.volumes.map((v) => v.filesystem ?? 'Unknown').join(', ') || 'Unknown'
                    : '—'}
                </td>
                <td>
                  <code>
                    {p
                      ? p.volumes.flatMap((v) => v.mountPoints).join(', ') || 'Not available'
                      : '—'}
                  </code>
                </td>
                <td className="numeric">{formatCapacity(s.capacityBytes)}</td>
                <td className="numeric">
                  {p
                    ? p.volumes.length
                      ? p.volumes
                          .map((v) =>
                            v.sharedSpace
                              ? 'Shared · Unknown'
                              : `${formatCapacity(v.usedBytes)} / ${formatCapacity(v.freeBytes)}`,
                          )
                          .join(', ')
                      : 'Unknown'
                    : '—'}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
