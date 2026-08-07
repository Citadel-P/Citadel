import { render, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { MemoryRouter, Route, Routes } from 'react-router';
import {
  ApplyStackInput,
  StackApplyEventType,
  StackStreamItem,
  SwarmServiceProgressItem,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useResourceParamType, useStreamProgress } from './hooks';

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

function SwarmServiceProgressProbe() {
  const state = useStreamProgress<Record<string, never>, SwarmServiceProgressItem>({
    endpoint: 'api/v1/swarmServices/service-id/apply',
    request: {},
    successMessage: 'Service applied successfully',
    errorMessageDefault: 'Failed to apply Service',
    getError: (item) => item.errorMessage,
    getIsComplete: (item) => item.isCompleted === true,
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

  it('finishes when a terminal item arrives even if the server leaves the response stream open', async () => {
    const encoder = new TextEncoder();
    server.use(
      http.post('http://localhost/api/v1/swarmServices/service-id/apply', () =>
        new HttpResponse(
          new ReadableStream({
            start(controller) {
              controller.enqueue(
                encoder.encode(
                  JSON.stringify({
                    serviceId: '00000000-0000-0000-0000-000000000001',
                    operationId: '00000000-0000-0000-0000-000000000002',
                    stage: 'accepted',
                    message: 'Docker accepted the Service.',
                    isCompleted: true,
                  } satisfies SwarmServiceProgressItem),
                ),
              );
            },
          }),
          { headers: { 'Content-Type': 'application/json' } },
        ),
      ),
    );

    const view = renderCitadel(<SwarmServiceProgressProbe />);

    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('success'));
    expect(view.getByText('Docker accepted the Service.')).toBeVisible();
  });
});

function ResourceTypeProbe() {
  const { type } = useResourceParamType();
  return <div data-testid="resource-type">{type}</div>;
}

describe('useResourceParamType', () => {
  it('maps the managed Swarm Service route to its resource type', () => {
    const view = render(
      <MemoryRouter initialEntries={['/swarm-services/edit/service-id']}>
        <Routes>
          <Route path="/:type/edit/:id" element={<ResourceTypeProbe />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(view.getByTestId('resource-type')).toHaveTextContent('SwarmService');
  });
});
