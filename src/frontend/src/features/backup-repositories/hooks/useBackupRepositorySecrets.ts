import { BackupRepositoryView, ResourceType } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';

export type RepositorySecretTarget = Pick<BackupRepositoryView, 'id' | 'capabilities'>;

export function useBackupRepositorySecrets(repository?: RepositorySecretTarget | null) {
  const canList = !repository || repository.capabilities?.canWrite === true;
  const { data, isLoading } = useRead(
    'listSecretDefinitions',
    { query: { targetResourceType: ResourceType.BackupRepository, targetResourceId: repository?.id } },
    { enabled: canList },
  );
  return { canList, secrets: canList ? (data?.data.secrets ?? []) : [], isLoading: canList && isLoading };
}
