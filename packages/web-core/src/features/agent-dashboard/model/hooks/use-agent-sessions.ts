import { useQuery } from '@tanstack/react-query';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';
import type { AgentSession } from '../types';

async function fetchAgentSessions(agentId: string): Promise<AgentSession[]> {
  const response = await makeLocalApiRequest(
    `/api/agents/${agentId}/sessions`
  );
  if (!response.ok) {
    throw new Error(`Failed to fetch agent sessions: ${response.statusText}`);
  }
  return response.json();
}

export function useAgentSessions(agentId: string | null) {
  return useQuery({
    queryKey: ['agents', agentId, 'sessions'],
    queryFn: () => fetchAgentSessions(agentId!),
    enabled: !!agentId,
  });
}
