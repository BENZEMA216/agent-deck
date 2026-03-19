import { createFileRoute, Navigate } from '@tanstack/react-router';

function OnboardingSignInRouteComponent() {
  // Auth removed for local-only use — redirect to agents
  return <Navigate to="/agents" replace />;
}

export const Route = createFileRoute('/onboarding_/sign-in')({
  component: OnboardingSignInRouteComponent,
});
