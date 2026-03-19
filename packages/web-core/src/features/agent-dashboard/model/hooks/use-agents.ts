import { useQuery } from '@tanstack/react-query';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';
import type { AgentRecord } from '../types';

async function fetchAgents(): Promise<AgentRecord[]> {
  const response = await makeLocalApiRequest('/api/agents');
  if (!response.ok) {
    throw new Error(`Failed to fetch agents: ${response.statusText}`);
  }
  return response.json();
}

export function useAgents() {
  return useQuery({
    queryKey: ['agents'],
    queryFn: fetchAgents,
    refetchInterval: 30_000,
  });
}
