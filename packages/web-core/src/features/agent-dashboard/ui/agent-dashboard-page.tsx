import { useState } from 'react';
import { Plus } from 'lucide-react';
import { useAgents } from '../model/hooks/use-agents';
import { useAgentDashboardStore } from '../model/stores/dashboard-store';
import { AgentCard } from './agent-card';
import { AgentDetailPanel } from './agent-detail-panel';
import { AgentRegisterModal } from './agent-register-modal';

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
        <h1 className="text-xl font-medium text-high">Agent Dashboard</h1>
        <button
          type="button"
          onClick={() => setRegisterModalOpen(true)}
          className="flex items-center gap-1.5 px-base py-1.5 rounded border border-brand bg-brand/10 text-sm text-high hover:bg-brand/20 focus:outline-none focus:ring-1 focus:ring-brand"
        >
          <Plus className="w-4 h-4" />
          Add Agent
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-base">
        {isLoading && (
          <div className="flex items-center justify-center py-double">
            <span className="text-sm text-low">Loading agents...</span>
          </div>
        )}

        {isError && (
          <div className="flex flex-col items-center justify-center py-double gap-2">
            <span className="text-sm text-error">
              Failed to load agents
              {error instanceof Error ? `: ${error.message}` : '.'}
            </span>
          </div>
        )}

        {!isLoading && !isError && agents && agents.length === 0 && (
          <div className="flex flex-col items-center justify-center py-double gap-2">
            <span className="text-sm text-low">
              No agents registered yet.
            </span>
            <button
              type="button"
              onClick={() => setRegisterModalOpen(true)}
              className="text-sm text-brand hover:underline focus:outline-none"
            >
              Register your first agent
            </button>
          </div>
        )}

        {!isLoading && !isError && agents && agents.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-base">
            {agents.map((agent) => (
              <AgentCard
                key={agent.id}
                agent={agent}
                onClick={handleCardClick}
              />
            ))}
          </div>
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
