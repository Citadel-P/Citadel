import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import LoadingBar, { LoadingBarRef } from 'react-top-loading-bar';

const LoadingBarWrapper = () => {
  const ref = useRef<LoadingBarRef>(null);
  const client = useQueryClient();

  useEffect(() => {
    const showProgressBar = () => ref.current?.continuousStart();
    const hideProgressBar = () => ref.current?.complete();

    const unsubscribeQueries = client.getQueryCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'fetch') showProgressBar();
      else if (
        event.type === 'updated' &&
        (event.action.type === 'error' || event.action.type === 'failed' || event.action.type === 'success')
      ) {
        hideProgressBar();
      }
    });

    const unsubscribeMutations = client.getMutationCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'pending') showProgressBar();
      else if (
        event.type === 'updated' &&
        (event.action.type === 'error' || event.action.type === 'failed' || event.action.type === 'success')
      ) {
        hideProgressBar();
      }
    });

    return () => {
      unsubscribeQueries();
      unsubscribeMutations();
    };
  }, [client]);

  return <LoadingBar color="var(--primary)" ref={ref} shadow={true} />;
};

export default LoadingBarWrapper;
