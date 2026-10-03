import { useCallback, useLayoutEffect, useRef } from 'react';
import { matchPath, useLocation, useNavigate } from 'react-router';
import type { ResourceType } from '@/api/types';
import TaskSheet from '@/components/custom/task-sheet';
import { useTaskSheet } from '@/lib/atoms';

export function useOpenAlertEventSheet() {
  const { open } = useTaskSheet('Alert');

  return useCallback(
    (alertEventId: string) => {
      open({ kind: 'alertEvent', payload: { id: alertEventId } });
    },
    [open],
  );
}

export function AlertTaskSheet() {
  return <RouteAwareEventTaskSheet type="Alert" routePrefix="alerts" taskKind="alertEvent" />;
}

export function ActivityTaskSheet() {
  return <RouteAwareEventTaskSheet type="Activity" routePrefix="activities" taskKind="activity" />;
}

function RouteAwareEventTaskSheet({
  type,
  routePrefix,
  taskKind,
}: {
  type: Extract<ResourceType, 'Alert' | 'Activity'>;
  routePrefix: 'alerts' | 'activities';
  taskKind: 'alertEvent' | 'activity';
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const routeMatch = matchPath(`/${routePrefix}/:eventId`, location.pathname);
  const requestedEventId = routeMatch?.params.eventId;
  const { state, open, close } = useTaskSheet(type);
  const openEventId = state.task?.kind === taskKind ? state.task.payload.id : undefined;
  const routeOpenedEventIdRef = useRef<string | null>(null);
  const isClosingRef = useRef(false);

  useLayoutEffect(() => {
    if (requestedEventId) {
      routeOpenedEventIdRef.current = requestedEventId;
      if (!isClosingRef.current && (!state.open || openEventId !== requestedEventId)) {
        const task =
          taskKind === 'alertEvent'
            ? ({ kind: 'alertEvent', payload: { id: requestedEventId } } as const)
            : ({ kind: 'activity', payload: { id: requestedEventId } } as const);
        open(task);
      }
      return;
    }

    isClosingRef.current = false;
    if (routeOpenedEventIdRef.current) {
      routeOpenedEventIdRef.current = null;
      if (state.open && state.task?.kind === taskKind) close();
    }
  }, [close, open, openEventId, requestedEventId, state.open, state.task?.kind, taskKind]);

  const closeDeepLink = useCallback(() => {
    if (!requestedEventId) return;

    isClosingRef.current = true;
    navigate(`/${routePrefix}`, { replace: true });
  }, [navigate, requestedEventId, routePrefix]);

  return <TaskSheet type={type} onClose={closeDeepLink} />;
}
