import { useCallback, useEffect, useEffectEvent, useRef, useState } from 'react';

type Row = { id: string };
const markerLimit = 256;
type Collection<T> = {
  rows?: T[];
  deleted: ReadonlySet<string>;
  liveSnapshot?: boolean;
  generation?: number;
  recovering?: boolean;
  saturated?: boolean;
  refreshAttempt?: number;
};

// An ordinary query result can overlap a live deletion or come from a filter's cache.
export function collectionSnapshot<T extends Row>(previous: Collection<T>, snapshot?: T[]): Collection<T> {
  if (previous.liveSnapshot || previous.recovering) return previous;
  if (!snapshot) return { ...previous, rows: undefined };
  const known = previous.saturated ? new Set(previous.rows?.map((row) => row.id)) : undefined;
  const rows =
    !known && !previous.deleted.size
      ? snapshot
      : snapshot.filter((row) => !previous.deleted.has(row.id) && (!known || known.has(row.id)));
  return { ...previous, rows };
}

export function collectionEvent<T extends Row>(
  previous: Collection<T>,
  row: T,
  action: string,
  matches: (row: T) => boolean,
  merge?: (incoming: T, existing: T) => T,
): Collection<T> {
  let deleted = previous.deleted;
  if (action === 'delete') {
    if (previous.liveSnapshot) return { ...previous, rows: previous.rows?.filter((item) => item.id !== row.id) };
    const saturated = previous.saturated || deleted.size >= markerLimit;
    return {
      ...previous,
      rows: previous.rows?.filter((item) => item.id !== row.id),
      saturated,
      deleted: saturated || deleted.has(row.id) ? deleted : new Set([...deleted, row.id]),
    };
  }
  if (deleted.has(row.id)) {
    if (action !== 'create') return previous;
    const remaining = new Set(deleted);
    remaining.delete(row.id);
    deleted = remaining;
  }
  const rows = previous.rows;
  if (!rows) return { ...previous, deleted };
  const index = rows.findIndex((item) => item.id === row.id);
  if (previous.saturated && index < 0 && action !== 'create') return previous;
  if (!matches(row))
    return { ...previous, rows: index < 0 ? rows : rows.filter((item) => item.id !== row.id), deleted };
  if (index < 0) return { ...previous, rows: [...rows, row], deleted };
  const updated = [...rows];
  updated[index] = merge ? merge(row, rows[index]) : row;
  return { ...previous, rows: updated, deleted };
}

const acceptsScopedRow = () => true;

export function useCollectionRows<T extends Row>(
  snapshot: T[] | undefined,
  scope: string,
  reload: () => Promise<T[] | undefined>,
  merge?: (incoming: T, existing: T) => T,
) {
  const [state, setState] = useState<Collection<T>>({ rows: snapshot, deleted: new Set() });
  const [baseline, setBaseline] = useState(snapshot);
  const [previousScope, setPreviousScope] = useState(scope);
  if (scope !== previousScope) {
    setPreviousScope(scope);
    setBaseline(snapshot);
    setState({ rows: snapshot, deleted: new Set() });
  } else if (baseline !== snapshot) {
    setBaseline(snapshot);
    setState((current) => collectionSnapshot(current, snapshot));
  }

  // A scoped HTTP read started after these deletes can retire the markers. If a
  // delete overlaps that read, keep the fence and use the next fresh baseline.
  const onJoinedGroup = useCallback(() => {
    // The realtime provider already reconciles active lists on reconnect.
    setState((current) =>
      current.deleted.size ? { ...current, refreshAttempt: (current.refreshAttempt ?? 0) + 1 } : current,
    );
  }, []);
  const deletes = useRef(0);
  const refresh = !!state.saturated || state.deleted.size >= markerLimit || !!state.refreshAttempt;
  const missingHttpBaseline = useEffectEvent(() => snapshot === undefined);
  useEffect(() => {
    if (!refresh) return;
    let active = true;
    const fence = deletes.current;
    const reconcile = async () => {
      // React Query can reuse a pending initial GET even for an explicit refetch.
      // Wait it out before reading a baseline newer than the released subscription.
      if (state.recovering && missingHttpBaseline()) {
        await reload();
        if (!active) return;
      }
      const rows = await reload();
      if (!active) return;
      if (!rows) {
        setState((current) =>
          current.generation === state.generation
            ? { ...current, recovering: false, refreshAttempt: undefined }
            : current,
        );
        return;
      }
      setState((current) =>
        current.liveSnapshot || current.generation !== state.generation
          ? current
          : fence === deletes.current
            ? { ...current, rows, deleted: new Set(), recovering: false, saturated: false, refreshAttempt: undefined }
            : { ...current, refreshAttempt: (current.refreshAttempt ?? 0) + 1 },
      );
    };
    void reconcile();
    return () => {
      active = false;
    };
  }, [refresh, reload, scope, state.refreshAttempt, state.recovering, state.generation]);

  const onEvent = useCallback(
    (row: T, action: string) => {
      if (action === 'delete') deletes.current++;
      setState((current) => collectionEvent(current, row, action, acceptsScopedRow, merge));
    },
    [merge],
  );
  const onCollectionSnapshot = useCallback((active: boolean) => {
    setState((current) =>
      active
        ? { rows: [], deleted: new Set(), liveSnapshot: true, generation: (current.generation ?? 0) + 1 }
        : current.liveSnapshot
          ? { ...current, liveSnapshot: false, recovering: true, refreshAttempt: (current.refreshAttempt ?? 0) + 1 }
          : current,
    );
  }, []);
  const transformRows = useCallback((transform: (rows: T[] | undefined) => T[] | undefined) => {
    setState((current) => {
      const rows = transform(current.rows);
      return rows === current.rows ? current : { ...current, rows };
    });
  }, []);
  return [state.rows, onEvent, onJoinedGroup, onCollectionSnapshot, transformRows] as const;
}
