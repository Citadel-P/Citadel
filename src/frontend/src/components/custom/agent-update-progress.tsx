import { useEffect, useRef } from 'react';
import { toast } from 'sonner';
import { useRead } from '@/lib/hooks';
import { SheetHeader, SheetTitle, SheetDescription } from '@/components/ui/sheet';
import { AlertMessage } from './alert-message';

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
      <SheetHeader>
        <SheetTitle>Agent updates</SheetTitle>
        <SheetDescription>
          {operation?.finished ? 'Update results' : 'Progress is saved by Citadel. You can close this sheet.'}
        </SheetDescription>
      </SheetHeader>
      {!operation ? (
        <p role="status">{query.isError ? 'Progress could not be loaded.' : 'Loading progress…'}</p>
      ) : (
        operation.targets.map((result) => (
          <div key={result.target.id} className="space-y-1 border-b p-3 text-sm">
            <p className="font-medium">{result.target.name}</p>
            <p className="text-muted-foreground">
              {result.target.installedVersion} → {result.target.availableVersion} ·{' '}
              {result.phase === 'NotStarted' ? 'Not started' : result.phase}
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
