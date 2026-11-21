import { useCallback, useEffect, useRef, useState } from 'react';

type Row = { id: string };
const markerLimit = 256;
type Collection<T> = {
  rows?: T[];
  deleted: ReadonlySet<string>;
  saturated?: boolean;
  refreshAttempt?: number;
};

// An ordinary query result can overlap a live deletion or come from a filter's cache.
export function collectionSnapshot<T extends Row>(previous: Collection<T>, snapshot?: T[]): Collection<T> {
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

export function useCollectionRows<T extends Row>(
  snapshot: T[] | undefined,
  scope: string,
  matches: (row: T) => boolean,
  reload: () => Promise<T[] | undefined>,
  merge?: (incoming: T, existing: T) => T,
) {
  const [state, setState] = useState<Collection<T>>({ rows: snapshot, deleted: new Set() });
  const [baseline, setBaseline] = useState(snapshot);
  if (baseline !== snapshot) {
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
  useEffect(() => {
    if (!refresh) return;
    let active = true;
    const fence = deletes.current;
    void reload().then((rows) => {
      if (!active || !rows) return;
      setState((current) =>
        fence === deletes.current
          ? { ...current, rows, deleted: new Set(), saturated: false, refreshAttempt: undefined }
          : { ...current, refreshAttempt: (current.refreshAttempt ?? 0) + 1 },
      );
    });
    return () => {
      active = false;
    };
  }, [refresh, reload, scope, state.refreshAttempt]);

  const onEvent = useCallback(
    (row: T, action: string) => {
      if (action === 'delete') deletes.current++;
      setState((current) => collectionEvent(current, row, action, matches, merge));
    },
    [matches, merge],
  );
  return [state.rows, onEvent, onJoinedGroup] as const;
}
