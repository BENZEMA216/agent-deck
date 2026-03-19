import type { HealthStatus } from '../model/types';

interface HealthIndicatorProps {
  health: HealthStatus;
  className?: string;
}

export function HealthIndicator({ health, className = '' }: HealthIndicatorProps) {
  const baseClasses = 'inline-block w-2 h-2 rounded-full shrink-0';

  switch (health.status) {
    case 'healthy':
      return (
        <span
          className={`${baseClasses} bg-success animate-pulse ${className}`}
          title={`Healthy (${health.latency_ms}ms)`}
        />
      );
    case 'degraded':
      return (
        <span
          className={`${baseClasses} bg-yellow-500 ${className}`}
          title={`Degraded: ${health.reason}`}
        />
      );
    case 'unreachable':
      return (
        <span
          className={`${baseClasses} bg-error ${className}`}
          title={`Unreachable${health.last_seen ? ` (last seen: ${health.last_seen})` : ''}`}
        />
      );
  }
}
