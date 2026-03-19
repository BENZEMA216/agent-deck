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

function getConnectionLabel(agent: AgentRecord): string {
  const config = agent.connection_config;
  if (agent.agent_type === 'openclaw') {
    return (config.url as string) ?? 'ws://localhost:18789';
  }
  if (agent.agent_type === 'claude_code') {
    return (config.cli_path as string) ?? '~/.local/bin/claude';
  }
  if (agent.agent_type === 'codex') {
    return (config.cli_path as string) ?? '/opt/homebrew/bin/codex';
  }
  return (config.url as string) ?? (config.cli_path as string) ?? '--';
}

function formatLastChecked(updatedAt: string): string {
  const diff = Date.now() - new Date(updatedAt).getTime();
  if (diff < 0) return 'just now';
  const secs = Math.floor(diff / 1000);
  if (secs < 10) return 'just now';
  if (secs < 60) return `${secs}s ago`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  return `${Math.floor(secs / 3600)}h ago`;
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
      <div className="flex items-center gap-2 mb-1">
        {health && <HealthIndicator health={health} />}
        <span className="text-lg font-medium text-high truncate">
          {agent.display_name}
        </span>
        {!agent.enabled && (
          <span className="text-xs bg-error/20 text-error rounded px-1.5 py-0.5">
            off
          </span>
        )}
      </div>

      <div className="flex items-center gap-2 mb-1.5">
        <span className="text-xs font-ibm-plex-mono bg-panel rounded px-1.5 py-0.5 text-low border border-border">
          {typeLabel}
        </span>
      </div>

      <div className="font-ibm-plex-mono text-xs text-low truncate mb-2">
        {getConnectionLabel(agent)}
      </div>

      {health && (
        <div className="text-xs text-low mb-2">
          checked {formatLastChecked(agent.updated_at)}
        </div>
      )}

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
