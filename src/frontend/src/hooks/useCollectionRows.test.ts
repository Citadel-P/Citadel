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
  act(() => hook.result.current[2]());
  act(() => hook.result.current[1](row, 'delete'));
  hook.rerender({ snapshot: [row] });
  act(() => hook.result.current[1](row, 'update'));
  expect(hook.result.current[0]).toEqual([]);
  act(() => hook.result.current[2]());
  await waitFor(() => expect(hook.result.current[0]).toEqual([{ ...row, value: 2 }]));
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
  act(() => hook.result.current[3](true));
  act(() => hook.result.current[1](row, 'update'));
  hook.rerender({ snapshot: fresh });
  expect(hook.result.current[0]).toEqual([row]);
  act(() => hook.result.current[3](false));
  expect(reload).toHaveBeenCalledOnce();
  hook.rerender({ snapshot: fresh });
  expect(hook.result.current[0]).toEqual([row]);
  await act(async () => complete(fresh));
  expect(hook.result.current[0]).toEqual(fresh);
});

it('discards a capacity repair that completes after a new live snapshot or scope switch', async () => {
  const pending: ((rows: (typeof row)[]) => void)[] = [];
  const reload = () => new Promise<(typeof row)[]>((resolve) => pending.push(resolve));
  const hook = renderHook(({ group }) => useCollectionRows(undefined, group, reload), {
    initialProps: { group: 'prod' },
  });
  act(() => hook.result.current[3](true));
  act(() => hook.result.current[3](false));
  act(() => hook.result.current[3](true));
  await act(async () => pending.shift()!([row]));
  expect(hook.result.current[0]).toEqual([]);
  act(() => hook.result.current[3](false));
  hook.rerender({ group: 'stage' });
  await act(async () => pending.shift()!([row]));
  expect(hook.result.current[0]).toBeUndefined();
});

it('waits out an initial HTTP read already in flight when the live subscription is lost', async () => {
  const pending: ((rows: (typeof row)[]) => void)[] = [];
  const reload = () => new Promise<(typeof row)[]>((resolve) => pending.push(resolve));
  const initialProps: { snapshot: (typeof row)[] | undefined } = { snapshot: undefined };
  const hook = renderHook(({ snapshot }) => useCollectionRows(snapshot, 'prod', reload), { initialProps });
  act(() => hook.result.current[3](true));
  act(() => hook.result.current[1](row, 'delete'));
  act(() => hook.result.current[3](false));
  const stale = [row];
  hook.rerender({ snapshot: stale });
  await act(async () => pending.shift()!(stale));
  expect(hook.result.current[0]).toEqual([]);
  const fresh = [{ ...row, id: 'b' }];
  await act(async () => pending.shift()!(fresh));
  expect(hook.result.current[0]).toEqual(fresh);
});

it('supersedes the previous repair when a new snapshot starts and loses capacity in one render', async () => {
  const initial = [row];
  const pending: ((rows: (typeof row)[]) => void)[] = [];
  const reload = () => new Promise<(typeof row)[]>((resolve) => pending.push(resolve));
  const hook = renderHook(() => useCollectionRows(initial, 'prod', reload));
  act(() => hook.result.current[3](true));
  act(() => hook.result.current[3](false));
  act(() => {
    hook.result.current[3](true);
    hook.result.current[3](false);
  });
  expect(pending).toHaveLength(2);
  await act(async () => pending.shift()!(initial));
  expect(hook.result.current[0]).toEqual([]);
  const latest = [{ ...row, id: 'b' }];
  await act(async () => pending.shift()!(latest));
  expect(hook.result.current[0]).toEqual(latest);
});
