import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

/** Review the final form payload without executing or persisting the source. */
export function useConfigurationCopyReview<T>({
  sourceName,
  describe,
}: {
  sourceName: string;
  describe: (payload: T) => ReactNode;
}) {
  const [payload, setPayload] = useState<T | null>(null);
  const resolver = useRef<((confirmed: boolean) => void) | null>(null);
  useEffect(() => () => resolver.current?.(false), []);
  const finish = useCallback((confirmed: boolean) => {
    resolver.current?.(confirmed);
    resolver.current = null;
    setPayload(null);
  }, []);
  const confirmSave = useCallback(
    (next: T) =>
      new Promise<boolean>((resolve) => {
        resolver.current?.(false);
        resolver.current = resolve;
        setPayload(next);
      }),
    [],
  );

  return {
    confirmSave,
    review: (
      <Dialog open={payload !== null} onOpenChange={(open) => !open && finish(false)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Review configuration copy</DialogTitle>
            <DialogDescription>
              Copying from {sourceName}. The source will not change. The new resource will not run automatically.
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-[60vh] space-y-3 overflow-y-auto text-sm">{payload !== null && describe(payload)}</div>
          <DialogFooter>
            <Button variant="outline" onClick={() => finish(false)}>
              Back to editing
            </Button>
            <Button onClick={() => finish(true)}>Create configuration copy</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    ),
  };
}
