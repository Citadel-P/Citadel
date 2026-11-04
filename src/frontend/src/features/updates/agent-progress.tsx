import { useEffect, useRef } from 'react';
import { toast } from 'sonner';
import { useRead } from '@/lib/hooks';
import { DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { StateBadge } from '@/components/custom/state-badge';
import { AlertMessage } from '@/components/custom/alert-message';

export function AgentUpdateProgress({ payload }: { payload: { id: string } }) {
  const query = useRead('getAgentUpdateOperation', { id: payload.id });
  const operation = query.data?.data;
  const followed = useRef(false);
  const notified = useRef(false);
  useEffect(() => {
    followed.current = false;
    notified.current = false;
  }, [payload.id]);
  useEffect(() => {
    if (!operation) return;
    if (!operation.finished) {
      followed.current = true;
      return;
    }
    if (!followed.current || notified.current) return;
    notified.current = true;
    if (operation.targets.some((t) => t.phase === 'Failed'))
      toast.error('Agent update failed', { description: 'Review the target results for recovery guidance.' });
    else if (operation.targets.some((t) => t.phase === 'NotStarted' || t.phase === 'Skipped'))
      toast.warning('Agent updates finished with skipped targets', { description: 'Review the target results.' });
    else toast.success('Agent updates finished');
  }, [operation]);
  return (
    <div className="space-y-4">
      <DialogHeader>
        <DialogTitle>Agent updates</DialogTitle>
        <DialogDescription>
          {operation?.finished ? 'Update results' : 'Progress is saved by Citadel. You can close this dialog.'}
        </DialogDescription>
      </DialogHeader>
      {!operation ? (
        <p role="status">{query.isError ? 'Progress could not be loaded.' : 'Loading progress…'}</p>
      ) : (
        operation.targets.map((result) => (
          <div key={result.target.id} className="space-y-1 border-b p-3 text-sm">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="font-medium">{result.target.name}</p>
              <StateBadge
                value={['Updating', 'Draining'].includes(result.phase) ? 'Running' : result.phase}
                kind="run"
                label={result.phase === 'NotStarted' ? 'Not started' : result.phase}
              />
            </div>
            <p className="text-muted-foreground">
              {result.target.installedVersion} → {result.target.availableVersion}
            </p>
            {result.message &&
              (result.phase === 'Failed' ? (
                <AlertMessage title="Update failed" type="error">
                  {result.message}
                </AlertMessage>
              ) : (
                <p className="text-muted-foreground">{result.message}</p>
              ))}
          </div>
        ))
      )}
    </div>
  );
}
