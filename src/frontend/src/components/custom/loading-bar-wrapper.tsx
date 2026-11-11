import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import LoadingBar, { LoadingBarRef } from 'react-top-loading-bar';

const IDLE_DELAY_MS = 150;
const MIN_VISIBLE_MS = 400;

const LoadingBarWrapper = () => {
  const ref = useRef<LoadingBarRef>(null);
  const onFinished = useRef(() => {});
  const onReset = useRef(() => {});
  const [cycle, setCycle] = useState(0);
  const client = useQueryClient();

  useEffect(() => {
    let phase: 'idle' | 'loading' | 'finishing' = 'idle';
    let startedAt = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const isBusy = () =>
      client.isFetching() > 0 || client.isMutating({ predicate: (mutation) => !mutation.state.isPaused }) > 0;
    const clearTimer = () => {
      clearTimeout(timer);
      timer = undefined;
    };
    const sync = () => {
      // The library's completion animation resets its own progress. Wait for it
      // to finish before starting another cycle, even if requests arrive meanwhile.
      if (phase === 'finishing') return;
      if (isBusy()) {
        if (phase === 'loading') {
          clearTimer();
        } else if (timer === undefined) {
          // Keep this start even if the request settles immediately. Deferring
          // one task also lets the library finish resetting its previous cycle.
          timer = setTimeout(() => {
            timer = undefined;
            phase = 'loading';
            startedAt = Date.now();
            ref.current?.continuousStart();
            sync();
          }, 0);
        }
      } else if (phase === 'loading' && timer === undefined) {
        timer = setTimeout(
          () => {
            timer = undefined;
            if (isBusy()) return;
            phase = 'finishing';
            ref.current?.complete();
          },
          Math.max(IDLE_DELAY_MS, MIN_VISIBLE_MS - (Date.now() - startedAt)),
        );
      }
    };

    onFinished.current = () => {
      // A continuous tick can repaint the library's bar after completion when
      // React batches delayed timers. Discard that animation instance, and keep
      // the finishing phase until the replacement has committed its ref.
      setCycle((value) => value + 1);
    };
    onReset.current = () => {
      phase = 'idle';
      sync();
    };
    const unsubscribeQueries = client.getQueryCache().subscribe(sync);
    const unsubscribeMutations = client.getMutationCache().subscribe(sync);
    // Include requests that started before the wrapper mounted.
    sync();

    return () => {
      unsubscribeQueries();
      unsubscribeMutations();
      clearTimer();
      onFinished.current = () => {};
      onReset.current = () => {};
    };
  }, [client]);

  useEffect(() => {
    onReset.current();
  }, [cycle]);

  return (
    <LoadingBar
      key={cycle}
      color="var(--primary)"
      ref={ref}
      shadow={true}
      waitingTime={300}
      onLoaderFinished={() => onFinished.current()}
    />
  );
};

export default LoadingBarWrapper;
