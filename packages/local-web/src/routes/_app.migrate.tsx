import { createFileRoute, Navigate } from '@tanstack/react-router';

// Migration disabled for local-only use
function MigrateRedirect() {
  return <Navigate to="/agents" replace />;
}

export const Route = createFileRoute('/_app/migrate')({
  component: MigrateRedirect,
});
