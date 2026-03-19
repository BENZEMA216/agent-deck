import type { HealthStatus } from '../model/types';

interface HealthIndicatorProps {
  health: HealthStatus;
  className?: string;
  verbose?: boolean;
}

export function HealthIndicator({
  health,
  className = '',
  verbose = false,
}: HealthIndicatorProps) {
  const dotClasses = 'inline-block w-2 h-2 rounded-full shrink-0';

  switch (health.status) {
    case 'healthy':
      return (
        <span
          className={`inline-flex items-center gap-1.5 ${className}`}
          title={`Healthy (${health.latency_ms}ms)`}
        >
          <span className={`${dotClasses} bg-success`} />
          {verbose && (
            <span className="text-xs font-ibm-plex-mono text-success">
              healthy
              <span className="text-low ml-1">{health.latency_ms}ms</span>
            </span>
          )}
        </span>
      );
    case 'degraded':
      return (
        <span
          className={`inline-flex items-center gap-1.5 ${className}`}
          title={`Degraded: ${health.reason}`}
        >
          <span className={`${dotClasses} bg-yellow-500`} />
          {verbose && (
            <span className="text-xs font-ibm-plex-mono text-yellow-500">
              degraded
              <span className="text-low ml-1">{health.reason}</span>
            </span>
          )}
        </span>
      );
    case 'unreachable':
      return (
        <span
          className={`inline-flex items-center gap-1.5 ${className}`}
          title={`Unreachable${health.last_seen ? ` (last seen: ${health.last_seen})` : ''}`}
        >
          <span className={`${dotClasses} bg-error`} />
          {verbose && (
            <span className="text-xs font-ibm-plex-mono text-error">
              offline
              {health.last_seen && (
                <span className="text-low ml-1">
                  last seen {health.last_seen}
                </span>
              )}
            </span>
          )}
        </span>
      );
  }
}
