import { ServiceAccountView } from '@/api/generated/api.types';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation } from 'react-router';
import { ServiceAccountInfoActions } from './actions';

const mocks = vi.hoisted(() => ({
  mutateAsync: vi.fn(),
}));

vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useMutate: () => ({ mutateAsync: mocks.mutateAsync, isPending: false }),
}));

describe('Service Account actions', () => {
  it('archives the selected account and returns to the Service Accounts tab', async () => {
    const user = userEvent.setup();
    const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    const Archive = ServiceAccountInfoActions.archive;
    const account = {
      id: '019fefb8-caa7-78b8-80a9-ff472095a7d5',
      name: 'build-runner',
      archivedAtUtc: null,
    } as ServiceAccountView;

    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[`/access/service-accounts/edit/${account.id}`]}>
          <Archive resource={account} />
          <CurrentPath />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.click(screen.getByRole('button', { name: 'Archive' }));
    await user.type(screen.getByRole('textbox', { name: `Enter ${account.name} to confirm` }), account.name);
    await user.click(screen.getByRole('button', { name: 'Archive' }));

    expect(mocks.mutateAsync).toHaveBeenCalledWith({ data: { ids: [account.id] } });
    await waitFor(() => expect(screen.getByTestId('current-path')).toHaveTextContent('/access/service-accounts'));
  });
});

const CurrentPath = () => <span data-testid="current-path">{useLocation().pathname}</span>;
