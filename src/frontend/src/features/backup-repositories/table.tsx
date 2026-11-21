import {
  BackupExecutionLocation,
  BackupRepositorySpec,
  BackupRepositoryType,
  BackupRepositoryView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { IntegrationCard } from '@/components/custom/common';
import { DropdownActionButton, RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { StateIndicator } from '@/components/custom/state-indicator';
import { hasCapability } from '@/lib/resource-capabilities';
import { cn } from '@/lib/utils';
import { ActionData } from '@/pages/types';
import { CheckCircle2, Cloud, DatabaseBackup, Eye, FolderLock, LoaderCircle, RefreshCw, Scissors } from 'lucide-react';
import { type FC, type ReactNode } from 'react';
import { useNavigate } from 'react-router';

import { useRepositoryOperations, type RepositoryOperation } from './operations';

type BackupRepositoryActions = Record<
  string,
  FC<{ resource: BackupRepositoryView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
>;

export function BackupRepositoriesTable({
  items,
  actions,
  isLoading,
}: {
  items: BackupRepositoryView[];
  isLoading: boolean;
  actions: BackupRepositoryActions;
}) {
  if (isLoading) {
    return (
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
        {Array.from({ length: 8 }).map((_, index) => (
          <div key={index} className="h-35 animate-pulse rounded-xl border bg-muted/20" />
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
      {items.map((repository) => (
        <RepositoryCard key={repository.id} repository={repository} actions={actions} />
      ))}
    </div>
  );
}

export function RepositoryCard({
  repository,
  actions,
  onEdit,
}: {
  onEdit?: () => void;
  repository: BackupRepositoryView;
  actions: BackupRepositoryActions;
}) {
  const navigate = useNavigate();
  const { canRun, run, pending: pendingOperation } = useRepositoryOperations(repository);
  const Icon = repository.type === BackupRepositoryType.S3Compatible ? Cloud : FolderLock;
  const label = repository.type === BackupRepositoryType.S3Compatible ? 'S3-compatible' : 'Filesystem';
  const {
    edit: _routeEdit,
    validate: _validateAction,
    initialize: _initializeAction,
    check: _checkAction,
    prune: _pruneAction,
    ...repositoryActions
  } = actions;
  const canRead = hasCapability(repository, 'canRead');
  const isResourceProcessing = repository.controlState === ResourceControlState.Processing;

  const openEdit = () => {
    if (!canRead) return;
    if (onEdit) onEdit();
    else navigate(`/backup-repositories/edit/${repository.id}`);
  };

  const operationAction = (operation: RepositoryOperation, title: string, icon: ReactNode) => (
    <DropdownActionButton
      title={title}
      icon={icon}
      loading={pendingOperation === operation}
      disabled={!canRun(operation)}
      onClick={() => run(operation)}
    />
  );

  const cardActions = {
    edit: () => (
      <DropdownActionButton title="Edit" icon={<Eye className="h-4 w-4" />} disabled={!canRead} onClick={openEdit} />
    ),
    validate: () => operationAction('validate', 'Validate', <CheckCircle2 className="h-4 w-4" />),
    initialize: () => operationAction('initialize', 'Initialize', <DatabaseBackup className="h-4 w-4" />),
    check: () => operationAction('check', 'Check', <RefreshCw className="h-4 w-4" />),
    prune: () => operationAction('prune', 'Prune', <Scissors className="h-4 w-4" />),
    ...repositoryActions,
  };
  const progressLabel = pendingOperation
    ? {
        validate: 'Validating',
        initialize: 'Initializing',
        check: 'Checking',
        prune: 'Pruning',
      }[pendingOperation]
    : isResourceProcessing
      ? 'Processing'
      : null;

  return (
    <IntegrationCard
      title={repository.name}
      subtitle={destinationText(repository.spec)}
      disabled={!canRead}
      onEdit={openEdit}
      icon={
        <div
          className={cn(
            'flex h-10 w-10 items-center justify-center rounded-lg border shadow-sm',
            repository.type === BackupRepositoryType.S3Compatible
              ? 'border-sky-500/15 bg-sky-500/10 text-sky-600'
              : 'border-emerald-500/15 bg-emerald-500/10 text-emerald-600',
          )}>
          <Icon className="h-4 w-4" />
        </div>
      }
      footerLeft={
        progressLabel ? (
          <div className="flex min-w-0 items-center gap-1.5 text-muted-foreground">
            <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
            <span className="truncate text-xs font-medium">{progressLabel}</span>
          </div>
        ) : (
          <div className="flex min-w-0 items-center gap-1.5">
            <StateIndicator value={repository.status} />
            <span className="truncate text-xs font-medium text-muted-foreground">{label}</span>
          </div>
        )
      }
      footerRight={<RowActionMenu resource={repository} actions={cardActions} />}
    />
  );
}

export const destinationText = (spec: BackupRepositorySpec) => {
  if (isFileSystemSpec(spec)) {
    return spec.location === BackupExecutionLocation.Platform ? `Platform: ${spec.path}` : `Core: ${spec.path}`;
  }

  if (isS3Spec(spec)) {
    return `${spec.endpoint.replace(/\/$/, '')}/${spec.bucket}${spec.prefix ? `/${spec.prefix}` : ''}`;
  }

  return '-';
};

const isFileSystemSpec = (spec: BackupRepositorySpec): spec is Extract<BackupRepositorySpec, { $type: 'FileSystem' }> =>
  spec?.$type === 'FileSystem';

const isS3Spec = (spec: BackupRepositorySpec): spec is Extract<BackupRepositorySpec, { $type: 'S3Compatible' }> =>
  spec?.$type === 'S3Compatible';
