import { useEffect, useRef, useState } from 'react';
import { CloudOff, LoaderCircle, RefreshCw, WifiOff } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import { cn } from '@/lib/utils';

const RECONNECTING_GRACE_PERIOD_MS = 1_500;

type VisibleConnectionState = 'reconnecting' | 'disconnected' | 'offline';

const statusContent: Record<VisibleConnectionState, { label: string; description: string }> = {
  reconnecting: {
    label: 'Live updates interrupted',
    description: 'Displayed information may be outdated. Reconnecting...',
  },
  disconnected: {
    label: 'Live updates unavailable',
    description: 'Displayed information may be outdated.',
  },
  offline: {
    label: "You're offline",
    description: 'Live updates are unavailable until your connection returns.',
  },
};

export function LiveConnectionIndicator() {
  const { liveConnectionState, interruptedAt, retryConnection } = useRealtimeContext();
  const [visibleInterruptionAt, setVisibleInterruptionAt] = useState<number>();
  const [retryPending, setRetryPending] = useState(false);
  const recoveryToastForRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    if (
      liveConnectionState !== 'reconnecting' ||
      interruptedAt === undefined ||
      visibleInterruptionAt === interruptedAt
    ) {
      return;
    }

    const timer = window.setTimeout(() => {
      setVisibleInterruptionAt(interruptedAt);
    }, RECONNECTING_GRACE_PERIOD_MS);

    return () => window.clearTimeout(timer);
  }, [interruptedAt, liveConnectionState, visibleInterruptionAt]);

  useEffect(() => {
    const isImmediatelyVisible =
      liveConnectionState === 'offline' ||
      liveConnectionState === 'disconnected' ||
      (liveConnectionState === 'connecting' && interruptedAt !== undefined) ||
      retryPending;
    if (!isImmediatelyVisible || interruptedAt === undefined || visibleInterruptionAt === interruptedAt) {
      return;
    }

    const timer = window.setTimeout(() => {
      setVisibleInterruptionAt(interruptedAt);
    }, 0);

    return () => window.clearTimeout(timer);
  }, [interruptedAt, liveConnectionState, retryPending, visibleInterruptionAt]);

  let visibleState: VisibleConnectionState | undefined;
  if (retryPending) {
    visibleState = 'disconnected';
  } else if (liveConnectionState === 'offline') {
    visibleState = 'offline';
  } else if (liveConnectionState === 'disconnected') {
    visibleState = 'disconnected';
  } else if (liveConnectionState === 'connecting' && interruptedAt !== undefined) {
    visibleState = 'reconnecting';
  } else if (liveConnectionState === 'reconnecting' && visibleInterruptionAt === interruptedAt) {
    visibleState = 'reconnecting';
  }

  useEffect(() => {
    if (
      liveConnectionState !== 'connected' ||
      visibleInterruptionAt === undefined ||
      recoveryToastForRef.current === visibleInterruptionAt
    ) {
      return;
    }

    recoveryToastForRef.current = visibleInterruptionAt;
    toast.success('Live updates restored');
    const timer = window.setTimeout(() => {
      setVisibleInterruptionAt(undefined);
    }, 0);
    return () => window.clearTimeout(timer);
  }, [liveConnectionState, visibleInterruptionAt]);

  const handleRetry = async () => {
    if (retryPending) {
      return;
    }

    setRetryPending(true);
    try {
      await retryConnection();
    } catch {
      // The persistent status remains visible and reflects the resulting state.
    } finally {
      setRetryPending(false);
    }
  };

  if (!visibleState) {
    return null;
  }

  const content = statusContent[visibleState];
  const isDisconnected = visibleState === 'disconnected';
  const isOffline = visibleState === 'offline';
  const Icon = isOffline ? WifiOff : isDisconnected ? CloudOff : LoaderCircle;

  const indicator = isDisconnected ? (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      disabled={retryPending}
      onClick={handleRetry}
      aria-label={`${content.label}. Retry live updates`}
      className="h-8 gap-1.5 border border-red-500/25 bg-red-500/10 px-2 text-xs font-normal text-red-700 hover:bg-red-500/15 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300">
      <Icon className="size-3.5" />
      <span className="hidden xl:inline">{retryPending ? 'Reconnecting...' : content.label}</span>
      {retryPending ? (
        <LoaderCircle className="size-3 animate-spin" />
      ) : (
        <RefreshCw className="hidden size-3 xl:block" />
      )}
    </Button>
  ) : (
    <span
      role="status"
      aria-label={`${content.label}. ${content.description}`}
      className={cn(
        'inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md border px-2 text-xs font-normal',
        isOffline
          ? 'border-red-500/25 bg-red-500/10 text-red-700 dark:text-red-400'
          : 'border-amber-500/30 bg-amber-500/10 text-amber-800 dark:text-amber-300',
      )}>
      <Icon className={cn('size-3.5', visibleState === 'reconnecting' && 'animate-spin')} />
      <span className="hidden xl:inline">{content.label}</span>
    </span>
  );

  return (
    <Tooltip>
      <TooltipTrigger asChild>{indicator}</TooltipTrigger>
      <TooltipContent side="bottom" align="end" className="max-w-72">
        <p className="font-medium">{content.label}</p>
        <p className="mt-0.5 text-primary-foreground/80">{content.description}</p>
        {isDisconnected && !retryPending ? <p className="mt-1">Select to retry.</p> : null}
      </TooltipContent>
    </Tooltip>
  );
}
