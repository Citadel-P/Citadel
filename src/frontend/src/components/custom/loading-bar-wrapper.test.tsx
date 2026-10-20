import { StrictMode } from 'react';
import { render } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import LoadingBarWrapper from './loading-bar-wrapper';

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
