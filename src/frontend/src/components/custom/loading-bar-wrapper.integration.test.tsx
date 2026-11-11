import { StrictMode } from 'react';
import { act, cleanup, render } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import LoadingBarWrapper from './loading-bar-wrapper';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  cleanup();
  vi.clearAllTimers();
  vi.useRealTimers();
});

async function advance(ms: number) {
  // Flush React between timer ticks, as the browser does between tasks.
  for (let remaining = ms; remaining > 0; remaining -= 50) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(Math.min(remaining, 50));
    });
  }
}

function mount() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  const { container } = render(
    <StrictMode>
      <QueryClientProvider client={client}>
        <LoadingBarWrapper />
      </QueryClientProvider>
    </StrictMode>,
  );
  const fill = () => container.firstElementChild!.firstElementChild as HTMLElement;
  return { client, fill };
}

it('shows fast requests and can start again after the real completion animation', async () => {
  const { client, fill } = mount();
  await act(async () => {
    await client.fetchQuery({ queryKey: ['fast'], queryFn: async () => 'done' });
  });
  await advance(1);
  expect(fill().style.width).not.toBe('0%');
  expect(fill().style.opacity).toBe('1');
  await advance(1200);
  expect(fill().style.width).toBe('0%');

  await act(async () => {
    await client
      .getMutationCache()
      .build(client, { mutationFn: async () => 'saved' })
      .execute(undefined);
  });
  await advance(1);
  expect(fill().style.width).not.toBe('0%');
  expect(fill().style.opacity).toBe('1');
  await advance(1200);
  expect(fill().style.width).toBe('0%');
});

it('resumes pending work after the real completion callback resets the bar', async () => {
  const { client, fill } = mount();
  await act(async () => {
    await client.fetchQuery({ queryKey: ['first'], queryFn: async () => 'done' });
  });
  await advance(600);
  let resolve!: (value: string) => void;
  const response = new Promise<string>((done) => {
    resolve = done;
  });
  let request!: Promise<string>;
  act(() => {
    request = client.fetchQuery({ queryKey: ['next'], queryFn: () => response });
  });
  await advance(600);
  expect(fill().style.width).not.toBe('0%');
  expect(fill().style.width).not.toBe('100%');
  expect(fill().style.opacity).toBe('1');
  await act(async () => {
    resolve('done');
    await request;
  });
  await advance(1200);
  expect(fill().style.width).toBe('0%');
});

it('stays hidden after completion when rendering is delayed across animation timers', async () => {
  const { client, fill } = mount();
  await act(async () => {
    await client.fetchQuery({ queryKey: ['delayed-render'], queryFn: async () => 'done' });
  });
  await advance(1);
  expect(fill().style.width).not.toBe('0%');
  // Busy/background tabs can delay React's commit across both completion and
  // continuous animation ticks instead of rendering between every timer.
  await act(async () => {
    await vi.advanceTimersByTimeAsync(5000);
  });
  expect(client.isFetching()).toBe(0);
  expect(fill().style.width).toBe('0%');
  await advance(2000);
  expect(fill().style.width).toBe('0%');
});

it('starts queued work on the new animation instance after a delayed reset', async () => {
  const { client, fill } = mount();
  await act(async () => {
    await client.fetchQuery({ queryKey: ['first'], queryFn: async () => 'done' });
  });
  await advance(600);
  let resolve!: (value: string) => void;
  const response = new Promise<string>((done) => {
    resolve = done;
  });
  let request!: Promise<string>;
  act(() => {
    request = client.fetchQuery({ queryKey: ['queued'], queryFn: () => response });
  });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(5000);
  });
  await advance(1);
  expect(client.isFetching()).toBe(1);
  expect(fill().style.width).not.toBe('0%');
  expect(fill().style.width).not.toBe('100%');
  expect(fill().style.opacity).toBe('1');
  await act(async () => {
    resolve('done');
    await request;
  });
  await advance(1500);
  expect(client.isFetching()).toBe(0);
  expect(fill().style.width).toBe('0%');
});
