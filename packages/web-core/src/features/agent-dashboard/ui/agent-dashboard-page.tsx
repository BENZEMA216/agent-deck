import { useState } from 'react';
import { Plus } from 'lucide-react';
import { useAgents } from '../model/hooks/use-agents';
import { useAgentDashboardStore } from '../model/stores/dashboard-store';
import { AgentCard } from './agent-card';
import { AgentDetailPanel } from './agent-detail-panel';
import { AgentRegisterModal } from './agent-register-modal';

function StatusSummary({
  agents,
}: {
  agents: { status?: { health: { status: string } } }[];
}) {
  const total = agents.length;
  const healthy = agents.filter(
    (a) => a.status?.health.status === 'healthy'
  ).length;
  const unreachable = agents.filter(
    (a) => a.status?.health.status === 'unreachable'
  ).length;
  const degraded = total - healthy - unreachable;

  const parts: string[] = [`${total} agent${total !== 1 ? 's' : ''}`];
  if (healthy > 0) parts.push(`${healthy} healthy`);
  if (degraded > 0) parts.push(`${degraded} degraded`);
  if (unreachable > 0) parts.push(`${unreachable} unreachable`);

  return (
    <div className="font-ibm-plex-mono text-xs text-low">
      {parts.join(' \u00b7 ')}
    </div>
  );
}

export function AgentDashboardPage() {
  const { data: agents, isLoading, isError, error } = useAgents();
  const [registerModalOpen, setRegisterModalOpen] = useState(false);

  const { selectedAgentId, detailPanelOpen, openDetailPanel, closeDetailPanel } =
    useAgentDashboardStore();

  const selectedAgent = agents?.find((a) => a.id === selectedAgentId) ?? null;

  const handleCardClick = (agentId: string) => {
    openDetailPanel(agentId);
  };

  return (
    <div className="flex flex-col h-full bg-primary">
      {/* Header */}
      <div className="flex items-center justify-between p-base border-b border-border">
        <h1 className="text-xl font-ibm-plex-mono font-medium text-high">
          Agent Deck
        </h1>
        <button
          type="button"
          onClick={() => setRegisterModalOpen(true)}
          className="flex items-center gap-1.5 px-base py-1.5 rounded border border-brand bg-brand/10 text-sm font-ibm-plex-mono text-high hover:bg-brand/20 focus:outline-none focus:ring-1 focus:ring-brand"
        >
          <Plus className="w-4 h-4" />
          Add
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-base">
        {isLoading && (
          <div className="py-double">
            <span className="text-sm font-ibm-plex-mono text-low">
              loading...
            </span>
          </div>
        )}

        {isError && (
          <div className="py-double">
            <span className="text-sm font-ibm-plex-mono text-error">
              error: {error instanceof Error ? error.message : 'failed to load agents'}
            </span>
          </div>
        )}

        {!isLoading && !isError && agents && agents.length === 0 && (
          <div className="flex flex-col items-center justify-center py-double gap-2">
            <span className="text-sm font-ibm-plex-mono text-low">
              no agents registered
            </span>
            <button
              type="button"
              onClick={() => setRegisterModalOpen(true)}
              className="text-sm font-ibm-plex-mono text-brand hover:underline focus:outline-none"
            >
              + add your first agent
            </button>
          </div>
        )}

        {!isLoading && !isError && agents && agents.length > 0 && (
          <>
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-base">
              {agents.map((agent) => (
                <AgentCard
                  key={agent.id}
                  agent={agent}
                  onClick={handleCardClick}
                />
              ))}
            </div>

            <div className="mt-base pt-base border-t border-border">
              <StatusSummary agents={agents} />
            </div>
          </>
        )}
      </div>

      {/* Detail Panel */}
      {detailPanelOpen && selectedAgent && (
        <AgentDetailPanel agent={selectedAgent} onClose={closeDetailPanel} />
      )}

      {/* Register Modal */}
      <AgentRegisterModal
        open={registerModalOpen}
        onClose={() => setRegisterModalOpen(false)}
      />
    </div>
  );
}
