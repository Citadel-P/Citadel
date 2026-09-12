import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { StackLogs } from './container-logs';

const { logViewerMock } = vi.hoisted(() => ({
  logViewerMock: vi.fn(),
}));

vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: vi.fn(() => ({ isLoading: false })),
}));

vi.mock('@/components/custom/common', () => ({
  LogViewer: (props: { containerFilters?: string[]; enableContainerFilter?: boolean }) => {
    logViewerMock(props);
    return <div>Log viewer</div>;
  },
}));

describe('StackLogs', () => {
  beforeEach(() => logViewerMock.mockClear());

  it('keeps container filtering enabled while container names are discovered from streamed log prefixes', () => {
    renderCitadel(<StackLogs stackId="stack-1" containers={[]} />);

    expect(screen.getByText('Log viewer')).toBeVisible();
    expect(logViewerMock).toHaveBeenCalledWith(
      expect.objectContaining({
        containerFilters: [],
        enableContainerFilter: true,
        emptyMessage: 'Waiting for logs…',
      }),
    );
  });

  it('passes all known container names to the shared multi-select filter', () => {
    renderCitadel(<StackLogs stackId="stack-1" containers={['web.1', 'worker.1']} />);

    expect(logViewerMock).toHaveBeenCalledWith(
      expect.objectContaining({
        containerFilters: ['web.1', 'worker.1'],
        enableContainerFilter: true,
      }),
    );
  });
});
