import { createFileRoute } from '@tanstack/react-router';
import { AgentDashboardPage } from '@/features/agent-dashboard';

export const Route = createFileRoute('/_app/agents')({
  component: AgentDashboardPage,
});
