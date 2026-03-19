import { create } from 'zustand';

interface AgentDashboardState {
  selectedAgentId: string | null;
  detailPanelOpen: boolean;
  // actions
  selectAgent: (id: string | null) => void;
  openDetailPanel: (agentId: string) => void;
  closeDetailPanel: () => void;
}

export const useAgentDashboardStore = create<AgentDashboardState>()(
  (set) => ({
    selectedAgentId: null,
    detailPanelOpen: false,

    selectAgent: (id) => set({ selectedAgentId: id }),

    openDetailPanel: (agentId) =>
      set({ selectedAgentId: agentId, detailPanelOpen: true }),

    closeDetailPanel: () =>
      set({ detailPanelOpen: false, selectedAgentId: null }),
  })
);
