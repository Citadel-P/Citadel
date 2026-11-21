import { useResourceReconciliation } from '@/hooks/useQueryReconciliation';

export function useReconcileScript() {
  return useResourceReconciliation('scripts', 'listScripts', 'script', 'getScript');
}
