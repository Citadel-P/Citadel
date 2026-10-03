import { screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import { renderCitadel } from '@/test/render-citadel';
import { BrowserEntry, FileTree } from '.';

describe('FileTree', () => {
  it('loads directories lazily and keeps folders before files', async () => {
    const loadDirectory = vi.fn(async (path: string) => ({
      entries:
        path === ''
          ? [entry('z.txt', 'z.txt', 'file'), entry('src', 'src', 'directory'), entry('a.txt', 'a.txt', 'file')]
          : [entry('app.ts', 'src/app.ts', 'file')],
      isTruncated: false,
    }));
    const { user } = renderCitadel(
      <FileTree queryKey={['files']} rootPath="" rootName="repo" loadDirectory={loadDirectory} />,
    );

    await screen.findByText('src');
    expect(loadDirectory).toHaveBeenCalledTimes(1);

    const labels = screen
      .getAllByRole('button')
      .map((button) => button.textContent)
      .filter(Boolean);
    expect(labels.indexOf('src')).toBeLessThan(labels.indexOf('a.txt'));
    expect(labels.indexOf('a.txt')).toBeLessThan(labels.indexOf('z.txt'));

    await user.click(screen.getByRole('button', { name: 'Expand src' }));

    expect(await screen.findByText('app.ts')).toBeVisible();
    expect(loadDirectory).toHaveBeenCalledWith('src', expect.any(AbortSignal));
  });

  it('reveals and selects an initial nested file', async () => {
    const selected = vi.fn();
    const loadDirectory = vi.fn(async (path: string) => ({
      entries:
        path === '' ? [entry('deploy', 'deploy', 'directory')] : [entry('compose.yaml', 'deploy/compose.yaml', 'file')],
      isTruncated: false,
    }));

    renderCitadel(
      <FileTree
        queryKey={['initial-file']}
        rootPath=""
        rootName="repo"
        initialPath="deploy/compose.yaml"
        loadDirectory={loadDirectory}
        onSelect={selected}
      />,
    );

    await waitFor(() =>
      expect(selected).toHaveBeenCalledWith(expect.objectContaining({ path: 'deploy/compose.yaml' })),
    );
  });
});

function entry(name: string, path: string, type: BrowserEntry['type']): BrowserEntry {
  return { name, path, type };
}
