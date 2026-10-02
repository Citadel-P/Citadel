import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { GitChangedPath, GitChangedPathStatus } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { GitRepositoryCompareAction } from './compare-dialog';

vi.mock('@/lib/monaco', () => ({
  MonacoDiff: ({ original, modified }: { original: string; modified: string }) => (
    <div data-testid="diff">
      <pre data-testid="before">{original}</pre>
      <pre data-testid="after">{modified}</pre>
    </div>
  ),
}));

const props = {
  repositoryId: 'repo',
  repositoryName: 'Infrastructure',
  baseCommitSha: 'a'.repeat(40),
  headCommitSha: 'b'.repeat(40),
};
const baseUrl = 'http://localhost/api/v1/gitRepositories/repo';

function mockComparison(files: GitChangedPath[], options: { truncated?: boolean; unavailable?: boolean } = {}) {
  const compare = vi.fn();
  const reads = vi.fn();
  server.use(
    http.get(`${baseUrl}/compare`, ({ request }) => {
      compare(Object.fromEntries(new URL(request.url).searchParams));
      return HttpResponse.json({ ...props, files, isTruncated: options.truncated ?? false });
    }),
    http.get(`${baseUrl}/files/content`, ({ request }) => {
      const query = Object.fromEntries(new URL(request.url).searchParams);
      reads(query);
      return HttpResponse.json({
        repositoryId: 'repo',
        commitSha: query.commitSha,
        path: query.path,
        type: 'File',
        size: 20,
        isBinary: !!options.unavailable,
        isTruncated: false,
        content: options.unavailable
          ? null
          : `${query.commitSha === props.baseCommitSha ? 'old' : 'new'} ${query.path}`,
        previewUnavailableReason: options.unavailable ? 'Binary files cannot be previewed.' : null,
      });
    }),
  );
  return { compare, reads };
}

it('loads changes only when opened and fetches only the selected file at immutable revisions', async () => {
  const { compare, reads } = mockComparison([
    { path: 'compose.yaml', status: GitChangedPathStatus.Modified },
    { path: 'new.txt', status: GitChangedPathStatus.Added },
  ]);
  const { user, queryClient } = renderCitadel(<GitRepositoryCompareAction {...props} watchPaths={['deploy/**']} />);
  expect(compare).not.toHaveBeenCalled();
  expect(reads).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'View changes' }));
  expect(await screen.findByTestId('before')).toHaveTextContent('old compose.yaml');
  expect(screen.getByTestId('after')).toHaveTextContent('new compose.yaml');
  expect(compare).toHaveBeenCalledWith({ baseCommitSha: props.baseCommitSha, headCommitSha: props.headCommitSha });
  expect(reads).toHaveBeenCalledTimes(2);
  expect(screen.getByText(/deploy\/\*\*/)).toBeVisible();
  await user.click(screen.getByRole('button', { name: /new.txt/ }));
  await waitFor(() => expect(screen.getByTestId('after')).toHaveTextContent('new new.txt'));
  expect(screen.getByTestId('before')).toBeEmptyDOMElement();
  expect(reads).toHaveBeenCalledTimes(3);
  await user.click(screen.getByRole('button', { name: 'Close' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  await waitFor(() =>
    expect(
      queryClient
        .getQueryCache()
        .getAll()
        .filter((query) => query.queryKey[0] === 'getGitRepositoryFileContent'),
    ).toHaveLength(0),
  );
});

it.each([
  [GitChangedPathStatus.Deleted, undefined, 'old file.txt', ''],
  [GitChangedPathStatus.Renamed, 'previous.txt', 'old previous.txt', 'new file.txt'],
  [GitChangedPathStatus.Copied, 'source.txt', 'old source.txt', 'new file.txt'],
] as const)('handles %s files without requesting missing versions', async (status, previousPath, before, after) => {
  const { reads } = mockComparison([{ path: 'file.txt', previousPath, status }]);
  const { user } = renderCitadel(<GitRepositoryCompareAction {...props} />);
  await user.click(screen.getByRole('button', { name: 'View changes' }));
  expect(await screen.findByTestId('before')).toHaveTextContent(before);
  expect(screen.getByTestId('after').textContent).toBe(after);
  expect(reads).toHaveBeenCalledTimes(status === GitChangedPathStatus.Deleted ? 1 : 2);
});

it('explains preview limits instead of displaying a misleading empty diff', async () => {
  mockComparison([{ path: 'binary.dat', status: GitChangedPathStatus.Modified }], {
    truncated: true,
    unavailable: true,
  });
  const { user } = renderCitadel(<GitRepositoryCompareAction {...props} />);
  await user.click(screen.getByRole('button', { name: 'View changes' }));
  expect(await screen.findByText('Binary files cannot be previewed.')).toBeVisible();
  expect(screen.getByText(/Some files are not shown/)).toBeVisible();
  expect(screen.queryByTestId('diff')).not.toBeInTheDocument();
});

it('shows repository authorization failures without requesting file contents', async () => {
  const { reads } = mockComparison([]);
  server.use(
    http.get(`${baseUrl}/compare`, () =>
      HttpResponse.json({ detail: 'Repository access denied.', status: 403 }, { status: 403 }),
    ),
  );
  const { user } = renderCitadel(<GitRepositoryCompareAction {...props} />);
  await user.click(screen.getByRole('button', { name: 'View changes' }));
  expect(await screen.findByText('You do not have permission to access this resource.')).toBeVisible();
  expect(reads).not.toHaveBeenCalled();
});

it('shows an empty comparison without requesting file contents', async () => {
  const { reads } = mockComparison([]);
  const { user } = renderCitadel(<GitRepositoryCompareAction {...props} />);
  await user.click(screen.getByRole('button', { name: 'View changes' }));
  expect(await screen.findByText('No file changes between these commits.')).toBeVisible();
  expect(reads).not.toHaveBeenCalled();
});
