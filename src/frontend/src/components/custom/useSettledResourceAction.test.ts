import { act, renderHook } from '@testing-library/react';
import { useSettledResourceAction } from './useSettledResourceAction';

it.each([
  [true, true],
  [true, false],
  [false, false],
])('reconciles both mutations once the slower request settles (%s, %s)', async (a, b) => {
  const items = [{ id: 'a' }, { id: 'b' }];
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const completed = vi.fn();
  const mutate = vi.fn(async (item: (typeof items)[number]) => {
    if (item.id === 'b') await gate;
    if (!(item.id === 'a' ? a : b)) throw Error('rejected');
  });
  const hook = renderHook(() => useSettledResourceAction(items, mutate, completed));
  let done!: Promise<void>;
  act(() => {
    done = hook.result.current.run();
  });
  expect(hook.result.current.isPending).toBe(true);
  expect(completed).not.toHaveBeenCalled();
  await act(async () => {
    release();
    await done;
  });
  expect(mutate).toHaveBeenCalledTimes(2);
  expect(completed).toHaveBeenCalledOnce();
  expect(completed).toHaveBeenCalledWith(items.filter((_, i) => (i ? b : a)));
  expect(hook.result.current.isPending).toBe(false);
});
