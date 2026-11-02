import { StrictMode, useImperativeHandle } from 'react';
import type { IProps, LoadingBarRef } from 'react-top-loading-bar';
import type { Ref } from 'react';
import { act, cleanup, render } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import LoadingBarWrapper from './loading-bar-wrapper';

const bar = vi.hoisted(() => ({
  start: vi.fn(),
  complete: vi.fn(),
  finished: undefined as (() => void) | undefined,
}));

vi.mock('react-top-loading-bar', () => ({
  default: function MockLoadingBar({ ref, onLoaderFinished }: IProps & { ref: Ref<LoadingBarRef> }) {
    useImperativeHandle(ref, () => ({
      continuousStart: bar.start,
      complete: bar.complete,
      staticStart: vi.fn(),
      start: vi.fn(),
      increase: vi.fn(),
      decrease: vi.fn(),
      getProgress: () => 0,
    }));
    bar.finished = onLoaderFinished;
    return null;
  },
}));

function deferred() {
  let resolve!: (value: string) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<string>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

function mount(client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })) {
  const view = render(
    <QueryClientProvider client={client}>
      <LoadingBarWrapper />
    </QueryClientProvider>,
  );
  return { client, ...view };
}

beforeEach(() => {
  vi.useFakeTimers();
  bar.start.mockClear();
  bar.complete.mockClear();
});

afterEach(() => {
  cleanup();
  vi.clearAllTimers();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it('owns cache subscriptions across rerenders, Strict Mode and unmount', () => {
  const client = new QueryClient();
  const queryUnsubscribe = vi.fn();
  const mutationUnsubscribe = vi.fn();
  const queries = vi.spyOn(client.getQueryCache(), 'subscribe').mockReturnValue(queryUnsubscribe);
  const mutations = vi.spyOn(client.getMutationCache(), 'subscribe').mockReturnValue(mutationUnsubscribe);
  const tree = () => (
    <StrictMode>
      <QueryClientProvider client={client}>
        <LoadingBarWrapper />
      </QueryClientProvider>
    </StrictMode>
  );
  const view = render(tree());
  expect(queries).toHaveBeenCalledTimes(2);
  expect(mutations).toHaveBeenCalledTimes(2);
  expect(queryUnsubscribe).toHaveBeenCalledTimes(1);
  expect(mutationUnsubscribe).toHaveBeenCalledTimes(1);
  view.rerender(tree());
  expect(queries).toHaveBeenCalledTimes(2);
  expect(mutations).toHaveBeenCalledTimes(2);
  view.unmount();
  expect(queryUnsubscribe).toHaveBeenCalledTimes(2);
  expect(mutationUnsubscribe).toHaveBeenCalledTimes(2);
});

it('keeps one cycle running until overlapping queries and mutations all settle', async () => {
  const { client } = mount();
  const first = deferred();
  const second = deferred();
  const write = deferred();
  const requests = [
    client.fetchQuery({ queryKey: ['first'], queryFn: () => first.promise }),
    client.fetchQuery({ queryKey: ['second'], queryFn: () => second.promise }),
    client
      .getMutationCache()
      .build(client, { mutationFn: () => write.promise })
      .execute(undefined),
  ];
  await advance(200);
  expect(bar.start).toHaveBeenCalledTimes(1);
  first.resolve('done');
  second.resolve('done');
  await advance(600);
  expect(bar.complete).not.toHaveBeenCalled();
  write.resolve('done');
  await Promise.all(requests);
  await advance(150);
  expect(bar.complete).toHaveBeenCalledTimes(1);
  expect(bar.start).toHaveBeenCalledTimes(1);
});

it('shows fast requests for a minimum duration without reacting to cache-only updates', async () => {
  const { client } = mount();
  client.setQueryData(['cached'], 'snapshot');
  await advance(1000);
  expect(bar.start).not.toHaveBeenCalled();
  await client.fetchQuery({ queryKey: ['fast'], queryFn: async () => 'done' });
  await advance(1);
  expect(bar.start).toHaveBeenCalledTimes(1);
  await advance(200);
  expect(bar.complete).not.toHaveBeenCalled();
  await advance(200);
  expect(bar.complete).toHaveBeenCalledTimes(1);
});

it('keeps a minimum visible time and bridges short gaps between requests', async () => {
  const { client } = mount();
  const first = deferred();
  const request = client.fetchQuery({ queryKey: ['first'], queryFn: () => first.promise });
  await advance(150);
  first.resolve('done');
  await request;
  await advance(200);
  expect(bar.complete).not.toHaveBeenCalled();
  const next = deferred();
  const nextRequest = client.fetchQuery({ queryKey: ['next'], queryFn: () => next.promise });
  await advance(500);
  expect(bar.start).toHaveBeenCalledTimes(1);
  expect(bar.complete).not.toHaveBeenCalled();
  next.resolve('done');
  await nextRequest;
  await advance(100);
  const last = deferred();
  const lastRequest = client.fetchQuery({ queryKey: ['last'], queryFn: () => last.promise });
  await advance(200);
  expect(bar.complete).not.toHaveBeenCalled();
  last.resolve('done');
  await lastRequest;
  await advance(150);
  expect(bar.start).toHaveBeenCalledTimes(1);
  expect(bar.complete).toHaveBeenCalledTimes(1);
});

it('waits for the finishing animation before starting another cycle', async () => {
  const { client } = mount();
  const first = deferred();
  const request = client.fetchQuery({ queryKey: ['first'], queryFn: () => first.promise });
  await advance(600);
  first.resolve('done');
  await request;
  await advance(150);
  expect(bar.complete).toHaveBeenCalledTimes(1);
  const next = deferred();
  const nextRequest = client.fetchQuery({ queryKey: ['next'], queryFn: () => next.promise });
  await advance(600);
  expect(bar.start).toHaveBeenCalledTimes(1);
  act(() => bar.finished?.());
  await advance(150);
  expect(bar.start).toHaveBeenCalledTimes(2);
  next.resolve('done');
  await nextRequest;
  await advance(400);
  expect(bar.complete).toHaveBeenCalledTimes(2);
});

it('keeps running during retries and finishes after the final error', async () => {
  const { client } = mount();
  const queryFn = vi.fn().mockRejectedValue(new Error('unavailable'));
  const request = client.fetchQuery({ queryKey: ['retry'], queryFn, retry: 1, retryDelay: 1000 }).catch(() => {});
  await advance(800);
  expect(bar.start).toHaveBeenCalledTimes(1);
  expect(bar.complete).not.toHaveBeenCalled();
  await advance(200);
  await request;
  await advance(150);
  expect(queryFn).toHaveBeenCalledTimes(2);
  expect(bar.complete).toHaveBeenCalledTimes(1);
});

it('observes requests already running on mount and completes on cancellation', async () => {
  const client = new QueryClient();
  const request = client
    .fetchQuery({ queryKey: ['cancel'], queryFn: () => new Promise<string>(() => {}) })
    .catch(() => {});
  mount(client);
  await advance(600);
  expect(bar.start).toHaveBeenCalledTimes(1);
  await client.cancelQueries();
  await request;
  await advance(150);
  expect(bar.complete).toHaveBeenCalledTimes(1);
});

it('cleans pending animation timers on unmount', async () => {
  const { client, unmount } = mount();
  const request = client
    .fetchQuery({ queryKey: ['unmount'], queryFn: () => new Promise<string>(() => {}) })
    .catch(() => {});
  unmount();
  await advance(500);
  expect(bar.start).not.toHaveBeenCalled();
  await client.cancelQueries();
  await request;
  await advance(500);
  expect(bar.complete).not.toHaveBeenCalled();
});
