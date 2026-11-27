import { useRead } from '@/lib/hooks';

export function useCanManageDashboards() {
  const profile = useRead('getCurrentProfile', undefined, { staleTime: Infinity });
  return !profile.isError && !!profile.data && !profile.data.data.authentication?.isSharedAccount;
}
