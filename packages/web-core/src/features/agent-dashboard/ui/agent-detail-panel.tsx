import { useState } from 'react';
import { X } from 'lucide-react';
import type { AgentRecord, AgentSession } from '../model/types';
import { useAgentStatus } from '../model/hooks/use-agent-status';
import { useAgentSessions } from '../model/hooks/use-agent-sessions';
import { HealthIndicator } from './health-indicator';

interface AgentDetailPanelProps {
  agent: AgentRecord;
  onClose: () => void;
}

type Tab = 'overview' | 'sessions' | 'config';

const TAB_ITEMS: { id: Tab; label: string }[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'sessions', label: 'Sessions' },
  { id: 'config', label: 'Config' },
];

function SessionStatusBadge({ status }: { status: AgentSession['status'] }) {
  const colors: Record<AgentSession['status'], string> = {
    active: 'bg-success/20 text-success',
    idle: 'bg-yellow-500/20 text-yellow-500',
    completed: 'bg-panel text-low',
    failed: 'bg-error/20 text-error',
  };
  return (
    <span className={`text-xs rounded px-1.5 py-0.5 ${colors[status]}`}>
      {status}
    </span>
  );
}

function OverviewTab({ agent }: { agent: AgentRecord }) {
  const { data: status } = useAgentStatus(agent.id);
  const health = status?.health ?? agent.status?.health;

  return (
    <div className="space-y-4">
      {health && (
        <div>
          <h4 className="text-sm font-medium text-low mb-1">Health</h4>
          <div className="flex items-center gap-2">
            <HealthIndicator health={health} />
            <span className="text-base text-normal capitalize">
              {health.status}
            </span>
            {health.status === 'healthy' && (
              <span className="text-xs text-low">
                ({health.latency_ms}ms)
              </span>
            )}
            {health.status === 'degraded' && (
              <span className="text-xs text-low">{health.reason}</span>
            )}
          </div>
        </div>
      )}

      <div>
        <h4 className="text-sm font-medium text-low mb-1">Capabilities</h4>
        <div className="flex flex-wrap gap-1">
          {agent.capabilities.length === 0 && (
            <span className="text-xs text-low">None</span>
          )}
          {agent.capabilities.map((cap) => (
            <span
              key={cap}
              className="text-xs font-ibm-plex-mono bg-panel rounded px-1.5 py-0.5 text-normal"
            >
              {cap}
            </span>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <h4 className="text-sm font-medium text-low mb-0.5">
            Active sessions
          </h4>
          <span className="text-base text-high">
            {status?.active_sessions ?? agent.status?.active_sessions ?? 0}
          </span>
        </div>
        <div>
          <h4 className="text-sm font-medium text-low mb-0.5">Version</h4>
          <span className="text-base text-high font-ibm-plex-mono">
            {status?.version ?? agent.status?.version ?? '--'}
          </span>
        </div>
        <div>
          <h4 className="text-sm font-medium text-low mb-0.5">Uptime</h4>
          <span className="text-base text-high">
            {formatDuration(
              status?.uptime_secs ?? agent.status?.uptime_secs ?? null
            )}
          </span>
        </div>
        <div>
          <h4 className="text-sm font-medium text-low mb-0.5">Enabled</h4>
          <span className="text-base text-high">
            {agent.enabled ? 'Yes' : 'No'}
          </span>
        </div>
      </div>
    </div>
  );
}

function SessionsTab({ agentId }: { agentId: string }) {
  const { data: sessions, isLoading } = useAgentSessions(agentId);

  if (isLoading) {
    return <div className="text-sm text-low">Loading sessions...</div>;
  }

  if (!sessions || sessions.length === 0) {
    return <div className="text-sm text-low">No sessions found.</div>;
  }

  return (
    <div className="space-y-2">
      {sessions.map((session) => (
        <div
          key={session.id}
          className="bg-panel rounded border p-2 flex items-center justify-between"
        >
          <div className="min-w-0">
            <div className="text-sm text-high truncate">
              {session.name ?? session.id}
            </div>
            <div className="text-xs text-low">
              {new Date(session.created_at).toLocaleString()}
            </div>
          </div>
          <SessionStatusBadge status={session.status} />
        </div>
      ))}
    </div>
  );
}

function ConfigTab({ config }: { config: Record<string, unknown> }) {
  return (
    <pre className="bg-panel rounded border p-base text-xs font-ibm-plex-mono text-normal overflow-auto max-h-64">
      {JSON.stringify(config, null, 2)}
    </pre>
  );
}

function formatDuration(secs: number | null): string {
  if (secs == null) return '--';
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const parts: string[] = [];
  if (d > 0) parts.push(`${d}d`);
  if (h > 0) parts.push(`${h}h`);
  if (m > 0 || parts.length === 0) parts.push(`${m}m`);
  return parts.join(' ');
}

export function AgentDetailPanel({ agent, onClose }: AgentDetailPanelProps) {
  const [activeTab, setActiveTab] = useState<Tab>('overview');

  return (
    <div className="fixed inset-y-0 right-0 z-50 w-full max-w-md bg-primary border-l border-border shadow-lg flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-base border-b border-border">
        <h2 className="text-lg font-medium text-high truncate">
          {agent.display_name}
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="flex items-center justify-center w-6 h-6 rounded text-low hover:text-normal focus:outline-none focus:ring-1 focus:ring-brand"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-border">
        {TAB_ITEMS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveTab(tab.id)}
            className={`px-base py-2 text-sm transition-colors ${
              activeTab === tab.id
                ? 'text-high border-b-2 border-brand'
                : 'text-low hover:text-normal'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-base">
        {activeTab === 'overview' && <OverviewTab agent={agent} />}
        {activeTab === 'sessions' && <SessionsTab agentId={agent.id} />}
        {activeTab === 'config' && (
          <ConfigTab config={agent.connection_config} />
        )}
      </div>
    </div>
  );
}
