import { createFileRoute, Navigate } from '@tanstack/react-router';

function OnboardingLandingRouteComponent() {
  // Onboarding removed for local-only use — redirect to agents
  return <Navigate to="/agents" replace />;
}

export const Route = createFileRoute('/onboarding')({
  component: OnboardingLandingRouteComponent,
});
