export type AgentType = 'openclaw' | 'claude_code' | 'codex' | string;

export type HealthStatus =
  | { status: 'healthy'; latency_ms: number }
  | { status: 'degraded'; reason: string }
  | { status: 'unreachable'; last_seen: string | null };

export type AgentCapability =
  | 'CHAT'
  | 'TASK_EXECUTION'
  | 'CRON_SCHEDULING'
  | 'MULTI_CHANNEL'
  | 'SESSION_PERSIST'
  | 'HEALTH_ENDPOINT';

export interface AgentStatus {
  agent_type: AgentType;
  health: HealthStatus;
  active_sessions: number;
  uptime_secs: number | null;
  version: string | null;
  extra: Record<string, unknown>;
}

export interface AgentRecord {
  id: string;
  agent_type: AgentType;
  display_name: string;
  connection_config: Record<string, unknown>;
  capabilities: AgentCapability[];
  enabled: boolean;
  created_at: string;
  updated_at: string;
  status?: AgentStatus;
}

export interface AgentSession {
  id: string;
  name: string | null;
  status: 'active' | 'idle' | 'completed' | 'failed';
  created_at: string;
  updated_at: string | null;
}
