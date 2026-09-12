import { screen } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { LogViewer } from './common';

describe('LogViewer empty message', () => {
  it('shows a waiting message until streamed lines arrive', () => {
    const { rerender } = renderCitadel(<LogViewer logs={[]} emptyMessage="Waiting for logs…" />);

    expect(screen.getByText('Waiting for logs…')).toBeVisible();
    expect(screen.queryByText('No logs available...')).not.toBeInTheDocument();

    rerender(<LogViewer logs={[{ message: 'Container started' }]} emptyMessage="Waiting for logs…" />);

    expect(screen.getByText('Container started')).toBeVisible();
    expect(screen.queryByText('Waiting for logs…')).not.toBeInTheDocument();
  });

  it('retains the default message for other log viewers', () => {
    renderCitadel(<LogViewer logs={[]} />);
    expect(screen.getByText('No logs available...')).toBeVisible();
  });
});
