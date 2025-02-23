import { useQueryClient } from '@tanstack/react-query';
import { useRef } from 'react';
import LoadingBar from 'react-top-loading-bar';

const LoadingBarWrapper = () => {
  const ref = useRef(null);
  const client = useQueryClient();

  const showProgressBar = () => ref?.current?.continuousStart();
  const hideProgressBar = () => ref?.current?.complete();

  client.getQueryCache().subscribe((event) => {
    if (event.type === 'updated' && event.action.type === 'fetch') showProgressBar();
    else if (
      event.type === 'updated' &&
      (event.action.type === 'error' || event.action.type === 'failed' || event.action.type === 'success')
    ) {
      hideProgressBar();
    }
  });

  client.getMutationCache().subscribe((event) => {
    if (event.type === 'updated' && event.action.type === 'pending') showProgressBar();
    else if (
      event.type === 'updated' &&
      (event.action.type === 'error' || event.action.type === 'failed' || event.action.type === 'success')
    ) {
      hideProgressBar();
    }
  });

  return <LoadingBar color="var(--primary)" ref={ref} shadow={true} />;
};

export default LoadingBarWrapper;
