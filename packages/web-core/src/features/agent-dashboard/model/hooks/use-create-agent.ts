import { useMutation, useQueryClient } from '@tanstack/react-query';
import { makeLocalApiRequest } from '@/shared/lib/localApiTransport';
import type { AgentRecord, AgentType, AgentCapability } from '../types';

export interface CreateAgentPayload {
  agent_type: AgentType;
  display_name: string;
  connection_config: Record<string, unknown>;
  capabilities: AgentCapability[];
}

async function createAgent(payload: CreateAgentPayload): Promise<AgentRecord> {
  const response = await makeLocalApiRequest('/api/agents', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new Error(`Failed to create agent: ${response.statusText}`);
  }
  return response.json();
}

export function useCreateAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createAgent,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] });
    },
  });
}
