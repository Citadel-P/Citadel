import { useResourceReconciliation } from '@/hooks/useQueryReconciliation';

export function useReconcileBackupRepository() {
  return useResourceReconciliation(
    'backup-repositories',
    'listBackupRepositories',
    'backup-repository',
    'getBackupRepository',
  );
}
