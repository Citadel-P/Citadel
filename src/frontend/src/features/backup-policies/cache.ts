import { useResourceReconciliation } from '@/hooks/useQueryReconciliation';

export function useReconcileBackupPolicy() {
  return useResourceReconciliation('backup-policies', 'listBackupPolicies', 'backup-policy', 'getBackupPolicy');
}
