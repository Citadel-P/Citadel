import type { ResourceBackupSummaryView } from '@/api/generated/api.types';
import { ResourceBackupCell } from '@/features/backup-policies/table-coverage';

export function StackBackupCell({
  stackId,
  coverage,
}: {
  stackId: string;
  coverage?: ResourceBackupSummaryView | null;
}) {
  return (
    <ResourceBackupCell
      coverage={coverage}
      source={{ $type: 'Stack', stackId }}
      returnTo={`/stacks/edit/${stackId}/`}
    />
  );
}
