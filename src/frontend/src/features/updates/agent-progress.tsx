import { useEffect, useRef } from 'react';
import { toast } from 'sonner';
import { LoaderCircle } from 'lucide-react';
import { useRead } from '@/lib/hooks';
import { DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { StateBadge } from '@/components/custom/state-badge';
import { AlertMessage } from '@/components/custom/alert-message';

export function useAgentUpdateProgress(id?: string, approved = false) {
  const query = useRead('getAgentUpdateOperation', { id: id ?? '' }, { enabled: !!id });
  const operation = query.data?.data;
  const followed = useRef(false);
  const notified = useRef(false);
  useEffect(() => {
    followed.current = approved;
    notified.current = false;
  }, [id, approved]);
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
  }, [operation, id, approved]);
  return query;
}

export function AgentUpdateProgress({ query }: { query: ReturnType<typeof useAgentUpdateProgress> }) {
  const operation = query.data?.data;
  return (
    <div className="space-y-4">
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2">
          {operation && !operation.finished && (
            <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />
          )}
          Agent updates
        </DialogTitle>
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
                indicator={
                  !operation.finished && result.phase === 'Updating' ? (
                    <LoaderCircle aria-hidden="true" className="size-3 animate-spin motion-reduce:animate-none" />
                  ) : undefined
                }
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
