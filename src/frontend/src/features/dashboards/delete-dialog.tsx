import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { DashboardSummaryView } from '@/api/generated/api.types';
import { resourceQueryKey } from '@/api/dispatch';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { Trash } from 'lucide-react';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';

export function DashboardDeleteDialog({
  boards,
  group = false,
  close,
  onDeleted,
}: {
  boards: DashboardSummaryView[];
  group?: boolean;
  close: () => void;
  onDeleted?: () => void;
}) {
  const client = useQueryClient();
  const remove = useMutate('deleteDashboard');
  // Keep the identities and revisions the user reviewed, including across a list refetch.
  const [remaining, setRemaining] = useState(boards);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  async function confirm() {
    if (pending) return;
    setPending(true);
    setError('');
    let deleted = 0;
    try {
      for (const { id, revision } of remaining) {
        await remove.mutateAsync({ id, query: { revision } });
        deleted++;
        client.removeQueries({ queryKey: resourceQueryKey('getDashboard', { id }), exact: true });
      }
      onDeleted?.();
    } catch (cause) {
      const rest = remaining.slice(deleted);
      setRemaining(rest);
      setError(rest[0].name + ' — ' + requestErrorMessage(cause));
      throw cause;
    } finally {
      setPending(false);
      void client.invalidateQueries({ queryKey: ['listDashboards'] });
      if (deleted) void client.invalidateQueries({ queryKey: ['getDashboardDefault'] });
    }
  }
  return (
    <ActionWithDialog
      open
      renderTrigger={false}
      name={group ? 'Delete' : remaining[0].name}
      title="Delete"
      icon={<Trash className="size-4" />}
      variant="destructive"
      description="Deleting a dashboard removes its home preferences. Resources shown on it are kept."
      disabled={pending}
      onOpenChange={(open) => {
        if (!open && !pending) close();
      }}
      onClick={confirm}
      items={group ? remaining : undefined}
      additional={error && <p role="alert">{error}</p>}
    />
  );
}
