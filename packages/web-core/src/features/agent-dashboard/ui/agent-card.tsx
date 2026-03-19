import type { AgentRecord } from '../model/types';
import { HealthIndicator } from './health-indicator';

interface AgentCardProps {
  agent: AgentRecord;
  onClick: (agentId: string) => void;
}

const AGENT_TYPE_LABELS: Record<string, string> = {
  openclaw: 'OpenClaw',
  claude_code: 'Claude Code',
  codex: 'Codex',
};

function formatUptime(secs: number | null | undefined): string {
  if (secs == null) return '--';
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
  return `${Math.floor(secs / 86400)}d`;
}

export function AgentCard({ agent, onClick }: AgentCardProps) {
  const typeLabel =
    AGENT_TYPE_LABELS[agent.agent_type] ?? agent.agent_type;
  const health = agent.status?.health;
  const activeSessions = agent.status?.active_sessions ?? 0;
  const version = agent.status?.version;
  const uptime = agent.status?.uptime_secs;

  return (
    <button
      type="button"
      onClick={() => onClick(agent.id)}
      className="w-full text-left bg-secondary rounded border p-base hover:ring-1 hover:ring-brand focus:outline-none focus:ring-1 focus:ring-brand transition-shadow"
    >
      <div className="flex items-center gap-2 mb-2">
        {health && <HealthIndicator health={health} />}
        <span className="text-lg font-medium text-high truncate">
          {agent.display_name}
        </span>
      </div>

      <div className="flex items-center gap-2 mb-2">
        <span className="text-xs font-ibm-plex-mono bg-panel rounded px-1.5 py-0.5 text-low">
          {typeLabel}
        </span>
        {!agent.enabled && (
          <span className="text-xs bg-error/20 text-error rounded px-1.5 py-0.5">
            Disabled
          </span>
        )}
      </div>

      <div className="flex items-center justify-between text-sm text-low">
        <span>
          {activeSessions} session{activeSessions !== 1 ? 's' : ''}
        </span>
        <span className="flex items-center gap-2">
          {version && (
            <span className="font-ibm-plex-mono text-xs">v{version}</span>
          )}
          <span className="text-xs">up {formatUptime(uptime)}</span>
        </span>
      </div>
    </button>
  );
}
