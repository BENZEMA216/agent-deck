import { useQuery } from '@tanstack/react-query';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';
import type { AgentStatus } from '../types';

async function fetchAgentStatus(agentId: string): Promise<AgentStatus> {
  const response = await makeLocalApiRequest(`/api/agents/${agentId}/status`);
  if (!response.ok) {
    throw new Error(`Failed to fetch agent status: ${response.statusText}`);
  }
  return response.json();
}

export function useAgentStatus(agentId: string | null) {
  return useQuery({
    queryKey: ['agents', agentId, 'status'],
    queryFn: () => fetchAgentStatus(agentId!),
    enabled: !!agentId,
    refetchInterval: 10_000,
  });
}
