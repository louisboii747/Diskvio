import { formatCapacity, partitionSegments } from '@/lib/capacity'
import type { Disk } from '@/models/disk'
export function PartitionMap({ disk }: { disk: Disk }) {
  const segments = partitionSegments(disk)
  return (
    <figure className="partition-map">
      <figcaption>
        <strong>Drive layout</strong>
        <span>
          {disk.partitions.length} {disk.partitions.length === 1 ? 'partition' : 'partitions'}
        </span>
      </figcaption>
      <div
        className="partition-bar"
        role="img"
        aria-label={`Partition layout for ${disk.name}: ${segments.map((s) => `${s.partition ? s.name : 'Unassigned space'} ${formatCapacity(s.capacityBytes)} (${s.percent.toFixed(1)}%)`).join(', ')}`}
      >
        {segments.map((segment) => (
          <div
            key={segment.id}
            data-segment={segment.id}
            className={`partition-segment partition-color-${segment.colorIndex}`}
            style={{ width: `${segment.percent}%` }}
            title={`${segment.partition ? segment.name : 'Unassigned space'}: ${formatCapacity(segment.capacityBytes)}`}
          >
            {segment.percent > 12 && (
              <>
                <strong>{segment.name}</strong>
                <span>{formatCapacity(segment.capacityBytes)}</span>
              </>
            )}
          </div>
        ))}
      </div>
    </figure>
  )
}
