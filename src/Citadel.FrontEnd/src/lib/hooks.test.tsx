import { waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { ApplyStackInput, StackApplyEventType, StackStreamItem } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useStreamProgress } from './hooks';

const dockerConflict = 'Error response from daemon: Conflict. The container name is already in use.';
const stackRequest: ApplyStackInput = {
  id: '00000000-0000-0000-0000-000000000001',
  recreate: false,
};

function StackProgressProbe() {
  const state = useStreamProgress<ApplyStackInput, StackStreamItem>({
    endpoint: 'api/v1/stacks/apply',
    request: stackRequest,
    successMessage: 'Stack applied successfully',
    errorMessageDefault: 'Failed to deploy',
    compactDockerComposeOutput: true,
    getError: (item) => (item.exitCode !== 0 ? item.message : undefined),
  });

  return (
    <>
      <div data-testid="status">{state.status}</div>
      {state.logs.map((log, index) => (
        <div key={index}>{log.message}</div>
      ))}
    </>
  );
}

describe('useStreamProgress', () => {
  it('preserves a terminal stack error that immediately follows another stream item', async () => {
    server.use(
      http.post('http://localhost/api/v1/stacks/apply', () =>
        HttpResponse.json([
          {
            type: StackApplyEventType.StdOut,
            timestamp: new Date().toISOString(),
            progressMessage: 'Container application Creating',
            exitCode: 0,
          },
          {
            type: StackApplyEventType.StdErr,
            timestamp: new Date().toISOString(),
            message: dockerConflict,
            exitCode: 1,
          },
        ]),
      ),
    );

    const view = renderCitadel(<StackProgressProbe />);

    await waitFor(() => {
      expect(view.getByTestId('status')).toHaveTextContent('error');
    });
    expect(view.getByText(dockerConflict)).toBeVisible();
    expect(view.queryByText('Creating container application...')).not.toBeInTheDocument();
  });
});
