import { useCallback, useState } from 'react';
import type { ResourceBackupSummaryView, StackView } from '@/api/generated/api.types';
import { ResourceBackupCell } from '@/features/backup-policies/table-coverage';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function StackBackupCell({ stackId }: { stackId: string }) {
  const [snapshot, setSnapshot] = useState<{
    stackId: string;
    coverage?: ResourceBackupSummaryView | null;
  }>();
  const receive = useCallback(
    (stack: StackView, action: string) => {
      if (stack.id !== stackId) return;
      setSnapshot(action === 'delete' ? undefined : { stackId, coverage: stack.backupCoverage });
    },
    [stackId],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('StackInfoUpdated', receive),
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('StackInfoUpdated', receive),
    [receive],
  );
  // The existing, actor-authorized group supplies a full stack summary on join,
  // reconnect and backup invalidation. Child containers may represent only part
  // of the stack's storage, so their coverage cannot be used as its summary.
  const { isConnected } = useRealtimeGroup({
    groupName: `stack:${stackId}`,
    setupEventListeners,
    removeEventListeners,
  });
  return (
    <ResourceBackupCell
      coverage={isConnected && snapshot?.stackId === stackId ? snapshot.coverage : undefined}
      source={{ $type: 'Stack', stackId }}
      returnTo={`/stacks/edit/${stackId}/`}
    />
  );
}
