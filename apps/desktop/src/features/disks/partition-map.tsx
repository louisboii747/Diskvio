import { formatCapacity, partitionSegments } from '@/lib/capacity'
import type { Disk } from '@/models/disk'
export function PartitionMap({ disk }: { disk: Disk }) {
  const segments = partitionSegments(disk)
  return (
    <div className="partition-map">
      <div
        className="partition-bar"
        role="img"
        aria-label={`Partition layout for ${disk.name}: ${segments.map((s) => `${s.name} ${formatCapacity(s.capacityBytes)} (${s.percent.toFixed(1)}%)`).join(', ')}`}
      >
        {segments.map((segment) => (
          <div
            key={segment.id}
            data-segment={segment.id}
            className={`partition-segment partition-color-${segment.colorIndex}`}
            style={{ width: `${segment.percent}%` }}
            title={`${segment.name}: ${formatCapacity(segment.capacityBytes)}`}
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
      <div className="partition-legend">
        {segments.map((segment) => (
          <span key={segment.id}>
            <i className={`partition-color-${segment.colorIndex}`} />
            {segment.name}
            <small>{formatCapacity(segment.capacityBytes)}</small>
          </span>
        ))}
      </div>
      <p className="map-note">
        Segments are proportional to capacity. Small system partitions are listed below.
      </p>
    </div>
  )
}
