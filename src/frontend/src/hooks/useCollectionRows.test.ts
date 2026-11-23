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

it('keeps an authoritative scoped snapshot across a late HTTP read, then accepts fallback reads', () => {
  const reload = vi.fn(async () => [row]);
  const hook = renderHook(({ snapshot }) => useCollectionRows(snapshot, 'prod', reload), {
    initialProps: { snapshot: [row] },
  });
  act(() => hook.result.current[3](true));
  expect(hook.result.current[0]).toEqual([]);
  const incoming = { ...row, id: 'new' };
  act(() => hook.result.current[1](incoming, 'update'));
  hook.rerender({ snapshot: [row] });
  expect(hook.result.current[0]).toEqual([incoming]);
  act(() => hook.result.current[3](false));
  hook.rerender({ snapshot: [row] });
  expect(hook.result.current[0]).toEqual([row]);
  expect(reload).not.toHaveBeenCalled();
});
