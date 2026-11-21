import { BackupRepositoryStatus, ResourceControlState, type BackupRepositoryView } from '@/api/generated/api.types';
import { useMutate } from '@/lib/hooks';
import { useMutationState, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { getRepositoryOperationContext } from './input';

const operationKeys = {
  validate: 'validateBackupRepository',
  initialize: 'initializeBackupRepository',
  check: 'checkBackupRepository',
  prune: 'pruneBackupRepository',
} as const;
export type RepositoryOperation = keyof typeof operationKeys;
const messages = {
  validate: 'Repository validated',
  initialize: 'Repository initialized',
  check: 'Repository checked',
  prune: 'Repository pruned',
};

export function canOperateRepository(
  repository: BackupRepositoryView | null | undefined,
  operation: RepositoryOperation,
  locked = false,
) {
  if (
    !repository ||
    locked ||
    repository.capabilities?.canExecute !== true ||
    repository.controlState === ResourceControlState.Processing
  )
    return false;
  const ready = repository.status === BackupRepositoryStatus.Ready;
  return operation === 'validate' || (operation === 'initialize' ? !ready : ready);
}

export function useRepositoryOperations(repository: BackupRepositoryView | null | undefined) {
  const client = useQueryClient();
  const key = ['backup-repository-maintenance', repository?.id];
  const pending = useMutationState({
    filters: { mutationKey: key, status: 'pending' },
    select: (mutation) => mutation.options.mutationKey?.[2] as RepositoryOperation,
  })[0];
  const options = (operation: RepositoryOperation) => ({
    mutationKey: [...key, operation],
  });
  const validate = useMutate(operationKeys.validate, options('validate'));
  const initialize = useMutate(operationKeys.initialize, options('initialize'));
  const check = useMutate(operationKeys.check, options('check'));
  const prune = useMutate(operationKeys.prune, options('prune'));
  const canRun = (operation: RepositoryOperation, locked = false) =>
    canOperateRepository(repository, operation, locked || !!pending);
  const run = async (operation: RepositoryOperation, locked = false) => {
    if (!repository || !canRun(operation, locked) || client.isMutating({ mutationKey: key })) return;
    try {
      await { validate, initialize, check, prune }[operation].mutateAsync({
        id: repository.id,
        data: getRepositoryOperationContext(repository),
      });
      toast.success(messages[operation]);
    } catch {
      /* Shared mutation error feedback. */
    }
  };
  return { canRun, run, pending };
}
