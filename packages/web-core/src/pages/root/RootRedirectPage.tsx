import { useEffect } from 'react';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';

export function RootRedirectPage() {
  const appNavigation = useAppNavigation();

  useEffect(() => {
    appNavigation.goToAgents({ replace: true });
  }, [appNavigation]);

  return null;
}
