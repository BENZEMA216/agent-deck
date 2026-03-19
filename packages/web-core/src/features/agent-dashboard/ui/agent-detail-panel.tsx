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

function getConnectionInfo(agent: AgentRecord): { label: string; value: string } {
  const config = agent.connection_config;
  if (agent.agent_type === 'openclaw') {
    return {
      label: 'WebSocket URL',
      value: (config.url as string) ?? 'ws://localhost:18789',
    };
  }
  if (agent.agent_type === 'claude_code') {
    return {
      label: 'CLI Path',
      value: (config.cli_path as string) ?? '~/.local/bin/claude',
    };
  }
  if (agent.agent_type === 'codex') {
    return {
      label: 'CLI Path',
      value: (config.cli_path as string) ?? '/opt/homebrew/bin/codex',
    };
  }
  return {
    label: 'Connection',
    value: (config.url as string) ?? (config.cli_path as string) ?? '--',
  };
}

function OverviewTab({ agent }: { agent: AgentRecord }) {
  const { data: status } = useAgentStatus(agent.id);
  const health = status?.health ?? agent.status?.health;
  const conn = getConnectionInfo(agent);

  return (
    <div className="space-y-4">
      {/* Connection info */}
      <div>
        <h4 className="text-sm font-medium text-low mb-1">{conn.label}</h4>
        <div className="font-ibm-plex-mono text-sm text-high bg-secondary rounded border px-base py-1.5 truncate">
          {conn.value}
        </div>
      </div>

      {/* Health */}
      {health && (
        <div>
          <h4 className="text-sm font-medium text-low mb-1">Health</h4>
          <HealthIndicator health={health} verbose />
        </div>
      )}

      {/* Capabilities */}
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
    return <div className="text-sm font-ibm-plex-mono text-low">loading...</div>;
  }

  if (!sessions || sessions.length === 0) {
    return <div className="text-sm font-ibm-plex-mono text-low">no sessions</div>;
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

function ConfigTab({
  config,
  agentId,
}: {
  config: Record<string, unknown>;
  agentId: string;
}) {
  const { refetch, isFetching } = useAgentStatus(agentId);
  const [testResult, setTestResult] = useState<string | null>(null);

  const handleTestConnection = () => {
    setTestResult(null);
    refetch().then(
      (result) => {
        if (result.data) {
          setTestResult(`ok: ${result.data.health.status}`);
        } else if (result.error) {
          setTestResult(
            `error: ${result.error instanceof Error ? result.error.message : 'unknown'}`
          );
        }
      },
      (err: unknown) => {
        setTestResult(
          `error: ${err instanceof Error ? err.message : 'unknown'}`
        );
      }
    );
  };

  // Format JSON with syntax highlighting via classes
  const configJson = JSON.stringify(config, null, 2);

  return (
    <div className="space-y-4">
      <pre className="bg-secondary rounded border p-base text-xs font-ibm-plex-mono text-normal overflow-auto max-h-64 leading-relaxed">
        {configJson}
      </pre>

      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={handleTestConnection}
          disabled={isFetching}
          className="px-base py-1.5 rounded border border-brand bg-brand/10 text-xs font-ibm-plex-mono text-high hover:bg-brand/20 focus:outline-none focus:ring-1 focus:ring-brand disabled:opacity-50"
        >
          {isFetching ? 'testing...' : 'Test Connection'}
        </button>
        {testResult && (
          <span
            className={`text-xs font-ibm-plex-mono ${
              testResult.startsWith('ok') ? 'text-success' : 'text-error'
            }`}
          >
            {testResult}
          </span>
        )}
      </div>
    </div>
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
        <h2 className="text-lg font-ibm-plex-mono font-medium text-high truncate">
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
            className={`px-base py-2 text-sm font-ibm-plex-mono transition-colors ${
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
          <ConfigTab config={agent.connection_config} agentId={agent.id} />
        )}
      </div>
    </div>
  );
}
