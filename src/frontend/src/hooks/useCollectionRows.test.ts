import { act, renderHook, waitFor } from '@testing-library/react';
import { collectionEvent, collectionSnapshot, useCollectionRows } from './useCollectionRows';

const row = { id: 'a', visible: true, value: 1 };
const matches = (item: typeof row) => item.visible;

it('deduplicates creates, reconciles filters and preserves feature-specific merge rules', () => {
  let state = collectionSnapshot({ deleted: new Set<string>() }, [row]);
  state = collectionEvent(state, row, 'create', matches);
  state = collectionEvent(state, { ...row, value: 0 }, 'update', matches, (incoming, old) => ({
    ...incoming,
    value: Math.max(old.value, incoming.value),
  }));
  expect(state.rows).toEqual([row]);
  state = collectionEvent(state, { ...row, visible: false }, 'update', matches);
  expect(state.rows).toEqual([]);
  expect(collectionEvent(state, row, 'update', matches).rows).toEqual([row]);
});

it('guards deletes across cached snapshots and accepts a new scoped read on rejoin', async () => {
  const initial = [row];
  const reload = vi.fn(async () => [{ ...row, value: 2 }]);
  const hook = renderHook(({ snapshot }) => useCollectionRows(snapshot, 'all', reload), {
    initialProps: { snapshot: initial },
  });
  act(() => hook.result.current.onJoinedGroup());
  act(() => hook.result.current.onEvent(row, 'delete'));
  hook.rerender({ snapshot: [row] });
  act(() => hook.result.current.onEvent(row, 'update'));
  expect(hook.result.current.rows).toEqual([]);
  act(() => hook.result.current.onJoinedGroup());
  await waitFor(() => expect(hook.result.current.rows).toEqual([{ ...row, value: 2 }]));
  expect(reload).toHaveBeenCalledOnce();
});

it('renders a fresh HTTP result after capacity loss even when its array is structurally shared', async () => {
  const fresh = [{ ...row, id: 'b' }];
  let complete!: (rows: typeof fresh) => void;
  const reload = vi.fn(
    () =>
      new Promise<typeof fresh>((resolve) => {
        complete = resolve;
      }),
  );
  const hook = renderHook(({ snapshot }) => useCollectionRows(snapshot, 'prod', reload), {
    initialProps: { snapshot: [row] },
  });
  act(() => hook.result.current.onCollectionSnapshot(true));
  act(() => hook.result.current.onEvent(row, 'update'));
  hook.rerender({ snapshot: fresh });
  expect(hook.result.current.rows).toEqual([row]);
  act(() => hook.result.current.onCollectionSnapshot(false));
  expect(reload).toHaveBeenCalledOnce();
  hook.rerender({ snapshot: fresh });
  expect(hook.result.current.rows).toEqual([row]);
  await act(async () => complete(fresh));
  expect(hook.result.current.rows).toEqual(fresh);
});

it('discards a capacity repair that completes after a new live snapshot or scope switch', async () => {
  const pending: ((rows: (typeof row)[]) => void)[] = [];
  const reload = () => new Promise<(typeof row)[]>((resolve) => pending.push(resolve));
  const hook = renderHook(({ group }) => useCollectionRows(undefined, group, reload), {
    initialProps: { group: 'prod' },
  });
  act(() => hook.result.current.onCollectionSnapshot(true));
  act(() => hook.result.current.onCollectionSnapshot(false));
  act(() => hook.result.current.onCollectionSnapshot(true));
  await act(async () => pending.shift()!([row]));
  expect(hook.result.current.rows).toEqual([]);
  act(() => hook.result.current.onCollectionSnapshot(false));
  hook.rerender({ group: 'stage' });
  await act(async () => pending.shift()!([row]));
  expect(hook.result.current.rows).toBeUndefined();
});

it('waits out an initial HTTP read already in flight when the live subscription is lost', async () => {
  const pending: ((rows: (typeof row)[]) => void)[] = [];
  const reload = () => new Promise<(typeof row)[]>((resolve) => pending.push(resolve));
  const initialProps: { snapshot: (typeof row)[] | undefined } = { snapshot: undefined };
  const hook = renderHook(({ snapshot }) => useCollectionRows(snapshot, 'prod', reload), { initialProps });
  act(() => hook.result.current.onCollectionSnapshot(true));
  act(() => hook.result.current.onEvent(row, 'delete'));
  act(() => hook.result.current.onCollectionSnapshot(false));
  const stale = [row];
  hook.rerender({ snapshot: stale });
  await act(async () => pending.shift()!(stale));
  expect(hook.result.current.rows).toEqual([]);
  const fresh = [{ ...row, id: 'b' }];
  await act(async () => pending.shift()!(fresh));
  expect(hook.result.current.rows).toEqual(fresh);
});

it('supersedes the previous repair when a new snapshot starts and loses capacity in one render', async () => {
  const initial = [row];
  const pending: ((rows: (typeof row)[]) => void)[] = [];
  const reload = () => new Promise<(typeof row)[]>((resolve) => pending.push(resolve));
  const hook = renderHook(() => useCollectionRows(initial, 'prod', reload));
  act(() => hook.result.current.onCollectionSnapshot(true));
  act(() => hook.result.current.onCollectionSnapshot(false));
  act(() => {
    hook.result.current.onCollectionSnapshot(true);
    hook.result.current.onCollectionSnapshot(false);
  });
  expect(pending).toHaveLength(2);
  await act(async () => pending.shift()!(initial));
  expect(hook.result.current.rows).toEqual([]);
  const latest = [{ ...row, id: 'b' }];
  await act(async () => pending.shift()!(latest));
  expect(hook.result.current.rows).toEqual(latest);
});

it('keeps the live rows after a failed repair until a successful read of the same baseline', async () => {
  const cached = [{ ...row, id: 'b' }];
  const reload = vi.fn(async () => undefined);
  const initialProps: { receipt: number | undefined } = { receipt: 1 };
  const hook = renderHook(({ receipt }) => useCollectionRows(cached, 'prod', reload, receipt), { initialProps });
  act(() => hook.result.current.onCollectionSnapshot(true));
  act(() => hook.result.current.onEvent(row, 'update'));
  hook.rerender({ receipt: 2 });
  await act(async () => hook.result.current.onCollectionSnapshot(false));
  hook.rerender({ receipt: undefined });
  expect(hook.result.current.rows).toEqual([row]);
  expect(reload).toHaveBeenCalledTimes(1);
  hook.rerender({ receipt: 3 });
  expect(hook.result.current.rows).toBe(cached);
  expect(reload).toHaveBeenCalledTimes(1);
});
