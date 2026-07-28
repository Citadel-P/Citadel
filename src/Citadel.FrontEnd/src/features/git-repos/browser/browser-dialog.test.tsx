import { GitRepositoryView } from '@/api/generated/api.types';
import { createQueryClient } from '@/query-client-wrapper';
import { QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GitRepositoryBrowseAction } from './browser-dialog';

vi.mock('./repository-browser', () => ({
  RepositoryBrowser: ({ repository }: { repository: GitRepositoryView }) => (
    <div data-testid="repository-browser">{repository.name}</div>
  ),
}));

describe('GitRepositoryBrowseAction', () => {
  it('opens the repository browser in a dialog and clears browser queries when closed', async () => {
    const repository = {
      id: '019f9b44-a8da-7000-8000-000000000001',
      name: 'infrastructure',
    } as GitRepositoryView;
    const queryClient = createQueryClient();
    const treeKey = ['git-repository-browser', repository.id, 'commit'];
    const fileKey = ['getGitRepositoryFileContent', { id: repository.id, query: { path: 'compose.yaml' } }];
    queryClient.setQueryData(treeKey, { entries: [] });
    queryClient.setQueryData(fileKey, { content: 'services: {}' });
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={queryClient}>
        <GitRepositoryBrowseAction resource={repository} />
      </QueryClientProvider>,
    );

    expect(screen.queryByTestId('repository-browser')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Browse Repo' }));

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByTestId('repository-browser')).toHaveTextContent('infrastructure');

    await user.click(screen.getByRole('button', { name: 'Close' }));

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(queryClient.getQueryData(treeKey)).toBeUndefined();
    expect(queryClient.getQueryData(fileKey)).toBeUndefined();
  });
});
