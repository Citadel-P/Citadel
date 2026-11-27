import { createReadAdmission } from './read-admission';
it('admits four concurrent reads, drops a cancelled waiter, and releases slots after failures', async () => {
  const read = createReadAdmission(4);
  const signal = new AbortController();
  const releases: Array<() => void> = [];
  let active = 0,
    peak = 0;
  const blocked = () =>
    read(signal.signal, async () => {
      peak = Math.max(peak, ++active);
      await new Promise<void>((r) => releases.push(r));
      active--;
    });
  const running = Array.from({ length: 4 }, blocked);
  const cancelled = new AbortController();
  const removed = vi.fn();
  const waiter = read(cancelled.signal, removed);
  cancelled.abort();
  await expect(waiter).rejects.toThrow();
  const failure = read(signal.signal, async () => {
    throw new Error('fixture');
  });
  const last = blocked();
  await vi.waitFor(() => expect(releases).toHaveLength(4));
  releases.shift()!();
  await expect(failure).rejects.toThrow('fixture');
  await vi.waitFor(() => expect(releases).toHaveLength(4));
  releases.forEach((r) => r());
  await Promise.all([...running, last]);
  expect(removed).not.toHaveBeenCalled();
  expect(peak).toBe(4);
});
