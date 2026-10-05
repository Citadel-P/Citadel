import { useState } from 'react';
import { toast } from 'sonner';
import { render, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router';
import {
  ApplyStackInput,
  StackApplyEventType,
  StackStreamItem,
  SwarmServiceProgressItem,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useResourceParamType, useStreamProgress, useMutate, useSaveResource, useHTTPErrorHandler } from './hooks';

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
        <div key={index} data-severity={log.severity ?? 'info'}>
          {log.message}
        </div>
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
  it('does not toast a connection failure when Strict Mode restarts a successful stack stream', async () => {
    const notice = vi.spyOn(toast, 'error').mockImplementation(() => 'unused');
    server.use(
      http.post('http://localhost/api/v1/stacks/apply', () =>
        HttpResponse.json([{ type: 'CommandCompleted', progressMessage: 'Stack deployment completed.', exitCode: 0 }]),
      ),
    );
    function Errors() {
      useHTTPErrorHandler();
      return null;
    }
    const view = renderCitadel(
      <>
        <Errors />
        <StackProgressProbe />
      </>,
      { reactStrictMode: true },
    );
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('success'));
    expect(
      view.queryClient
        .getMutationCache()
        .getAll()
        .some((mutation) => mutation.state.error?.name === 'AbortError'),
    ).toBe(true);
    expect(notice).not.toHaveBeenCalled();
  });

  it('still reports an actual connection failure while starting a stack stream', async () => {
    const notice = vi.spyOn(toast, 'error').mockImplementation(() => 'unused');
    server.use(http.post('http://localhost/api/v1/stacks/apply', () => HttpResponse.error()));
    function Errors() {
      useHTTPErrorHandler();
      return null;
    }
    const view = renderCitadel(
      <>
        <Errors />
        <StackProgressProbe />
      </>,
    );
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('error'));
    expect(notice).toHaveBeenCalledWith('Request failed', {
      description: 'The request could not be completed. Check your connection and try again.',
    });
  });

  it('clears completed Compose activity from a single batch and recognizes Image prefixes', async () => {
    server.use(
      http.post('http://localhost/api/v1/stacks/apply', () =>
        HttpResponse.json([
          ...[
            'Network demo-app_default Creating',
            'Network demo-app_default Created',
            'Network demo-app_default Created',
            'Image example/demo-app:latest Pulling',
            'Image example/demo-app:latest Pulled',
            'Container demo-app Starting',
            'Container demo-app Started',
          ].map((progressMessage) => ({ type: 'StdErr', progressMessage })),
          {
            type: 'CommandCompleted',
            progressMessage: 'Stack deployment completed.',
            exitCode: 0,
            severity: 'success',
          },
        ]),
      ),
    );
    const view = renderCitadel(<StackProgressProbe />);
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('success'));
    expect(view.getByText('Pulled image example/demo-app:latest.')).toBeVisible();
    expect(view.getAllByText('Created network demo-app_default.')).toHaveLength(1);
    expect(view.queryByText('Creating network demo-app_default...')).not.toBeInTheDocument();
    expect(view.queryByText('Starting container demo-app...')).not.toBeInTheDocument();
    expect(view.queryByText(/Pulling image/)).not.toBeInTheDocument();
  });

  it('shows live chunks before completion and clears unfinished activity on completion', async () => {
    const encoder = new TextEncoder();
    let output: ReadableStreamDefaultController<Uint8Array>;
    server.use(
      http.post(
        'http://localhost/api/v1/stacks/apply',
        () =>
          new HttpResponse(
            new ReadableStream<Uint8Array>({
              start(controller) {
                output = controller;
                controller.enqueue(
                  encoder.encode('[{"type":"StdErr","progressMessage":"Network demo-app_default Creating"},'),
                );
              },
            }),
            { headers: { 'Content-Type': 'application/json' } },
          ),
      ),
    );
    const view = renderCitadel(<StackProgressProbe />);
    await waitFor(() => expect(view.getByText('Creating network demo-app_default...')).toBeVisible());
    expect(view.getByTestId('status')).toHaveTextContent('pending');
    output!.enqueue(
      encoder.encode(
        JSON.stringify({ type: 'CommandCompleted', exitCode: 0 }) +
          ',' +
          JSON.stringify({ type: 'StdErr', progressMessage: 'Container demo-app Starting' }) +
          ',',
      ),
    );
    await waitFor(() => expect(view.getByText('Starting container demo-app...')).toBeVisible());
    expect(view.queryByText('Creating network demo-app_default...')).not.toBeInTheDocument();
    output!.enqueue(
      encoder.encode(
        JSON.stringify({ type: 'CommandCompleted', progressMessage: 'Stack deployment completed.', exitCode: 0 }) + ']',
      ),
    );
    output!.close();
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('success'));
    expect(view.queryByText('Starting container demo-app...')).not.toBeInTheDocument();
    expect(view.getByText('Stack deployment completed.')).toBeVisible();
  });

  it('updates a single Swarm rollout row across chunks and retains each Service outcome', async () => {
    const encoder = new TextEncoder();
    let output: ReadableStreamDefaultController<Uint8Array>;
    const progress = (text: string) => JSON.stringify({ type: 'StdErr', progressMessage: text }) + ',';
    server.use(
      http.post(
        'http://localhost/api/v1/stacks/apply',
        () =>
          new HttpResponse(
            new ReadableStream<Uint8Array>({
              start(controller) {
                output = controller;
                controller.enqueue(
                  encoder.encode(
                    '[' +
                      progress(
                        'overall progress: 0 out of 1 tasks\r1/1: \r' +
                          'overall progress: 0 out of 1 tasks\r'.repeat(50),
                      ),
                  ),
                );
              },
            }),
            { headers: { 'Content-Type': 'application/json' } },
          ),
      ),
    );
    const view = renderCitadel(<StackProgressProbe />);
    await waitFor(() => expect(view.getAllByText('Swarm rollout: 0 of 1 tasks ready.')).toHaveLength(1));
    expect(view.queryByText('1/1:')).not.toBeInTheDocument();
    output!.enqueue(encoder.encode(progress('overall progress: 1 out of 1 tasks')));
    await waitFor(() => expect(view.getByText('Swarm rollout: 1 of 1 tasks ready.')).toBeVisible());
    expect(view.queryByText('Swarm rollout: 0 of 1 tasks ready.')).not.toBeInTheDocument();
    output!.enqueue(
      encoder.encode(progress('verify: Waiting 5 seconds to verify that tasks are stable...\n'.repeat(10))),
    );
    await waitFor(() => expect(view.getAllByText('Verifying task stability: 5s remaining...')).toHaveLength(1));
    expect(view.queryByText(/Swarm rollout:/)).not.toBeInTheDocument();
    output!.enqueue(encoder.encode(progress('verify: Waiting 1 seconds to verify that tasks are stable...')));
    await waitFor(() => expect(view.getByText('Verifying task stability: 1s remaining...')).toBeVisible());
    expect(view.queryByText('Verifying task stability: 5s remaining...')).not.toBeInTheDocument();
    output!.enqueue(
      encoder.encode(progress('verify: Service demo-web converged') + progress('overall progress: 0 out of 2 tasks')),
    );
    await waitFor(() => expect(view.getByText('Swarm rollout: 0 of 2 tasks ready.')).toBeVisible());
    expect(view.getByText('Service demo-web converged.')).toBeVisible();
    output!.enqueue(
      encoder.encode(
        progress('verify: Service demo-worker converged') +
          JSON.stringify({
            type: 'CommandCompleted',
            progressMessage: 'Stack deployment completed.',
            exitCode: 0,
          }) +
          ']',
      ),
    );
    output!.close();
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('success'));
    expect(view.getByText('Service demo-web converged.')).toBeVisible();
    expect(view.getByText('Service demo-worker converged.')).toBeVisible();
    expect(view.queryByText(/Swarm rollout:|Verifying task stability:/)).not.toBeInTheDocument();
  });

  it('preserves Swarm diagnostics and warnings while clearing failed rollout progress', async () => {
    const diagnostic = '1/1: no suitable node (insufficient resources)';
    server.use(
      http.post('http://localhost/api/v1/stacks/apply', () =>
        HttpResponse.json([
          { type: 'StdErr', progressMessage: 'overall progress: 0 out of 1 tasks' },
          { type: 'StdErr', progressMessage: diagnostic, severity: 'error' },
          { type: 'StdErr', progressMessage: 'overall progress: 0 out of 1 tasks', severity: 'warning' },
          { type: 'CommandCompleted', exitCode: 1, message: 'Stack deployment failed.' },
        ]),
      ),
    );
    const view = renderCitadel(<StackProgressProbe />);
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('error'));
    expect(view.getByText(diagnostic)).toHaveAttribute('data-severity', 'error');
    expect(view.getByText('overall progress: 0 out of 1 tasks')).toHaveAttribute('data-severity', 'warning');
    expect(view.getByText('Stack deployment failed.')).toHaveAttribute('data-severity', 'error');
    expect(view.queryByText(/Swarm rollout:/)).not.toBeInTheDocument();
  });

  it('renders normal Stack progress neutrally and successful completion in green', async () => {
    server.use(
      http.post('http://localhost/api/v1/stacks/apply', () =>
        HttpResponse.json([
          { type: 'SystemMessage', progressMessage: 'Resolving Stack variables and secrets...' },
          { type: 'StdErr', progressMessage: 'Container demo-app Started' },
          {
            type: 'CommandCompleted',
            progressMessage: 'Stack deployment completed.',
            exitCode: 0,
            stackStatus: 'Healthy',
            severity: 'success',
          },
        ]),
      ),
    );
    const view = renderCitadel(<StackProgressProbe />);
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('success'));
    expect(view.getByText('Resolving Stack variables and secrets...')).toHaveAttribute('data-severity', 'info');
    expect(view.getByText('Stack deployment completed.')).toHaveAttribute('data-severity', 'success');
    expect(view.container.querySelector('[data-severity="error"]')).toBeNull();
  });

  it('colors Compose errors without replaying normal progress in the terminal failure', async () => {
    const failure = 'Stack deployment failed (exit code 1).';
    server.use(
      http.post('http://localhost/api/v1/stacks/apply', () =>
        HttpResponse.json([
          { type: 'StdErr', progressMessage: 'demo-app Pulled' },
          { type: 'StdErr', progressMessage: 'Network demo-app-copy_default Created' },
          { type: 'StdErr', progressMessage: dockerConflict, severity: 'error' },
          { type: 'CommandCompleted', message: failure, exitCode: 1, severity: 'error' },
        ]),
      ),
    );
    const view = renderCitadel(<StackProgressProbe />);
    await waitFor(() => expect(view.getByTestId('status')).toHaveTextContent('error'));
    expect(view.getByText('Pulled image demo-app.')).toHaveAttribute('data-severity', 'info');
    expect(view.getByText('Created network demo-app-copy_default.')).toHaveAttribute('data-severity', 'info');
    expect(view.getAllByText(dockerConflict)).toHaveLength(1);
    expect(view.getByText(dockerConflict)).toHaveAttribute('data-severity', 'error');
    expect(view.getByText(failure)).toHaveAttribute('data-severity', 'error');
  });

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
    expect(view.getByText(dockerConflict)).toHaveAttribute('data-severity', 'error');
    expect(view.queryByText('Creating container application...')).not.toBeInTheDocument();
  });

  it('finishes when a terminal item arrives even if the server leaves the response stream open', async () => {
    const encoder = new TextEncoder();
    server.use(
      http.post(
        'http://localhost/api/v1/swarmServices/service-id/apply',
        () =>
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
                      isWarning: false,
                      errorMessage: null,
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

describe('registry save errors', () => {
  it.each(['add', 'edit'] as const)('does not show success or navigate after a rejected %s', async (mode) => {
    const success = vi.spyOn(toast, 'success').mockImplementation(() => 'unused');
    const message = 'Docker Hub rejected these credentials.';
    const reject = () =>
      HttpResponse.json({ title: 'Validation failed', status: 400, errors: { '': [message] } }, { status: 400 });
    server.use(
      http.post('http://localhost/api/v1/registries', reject),
      http.patch('http://localhost/api/v1/registries/registry-id', reject),
    );
    function Probe() {
      const create = useMutate('createRegistry');
      const update = useMutate('updateRegistry');
      const [failed, setFailed] = useState(false);
      const location = useLocation();
      const { save, isPending } = useSaveResource({
        mode,
        basePath: 'registries',
        entityName: 'Registry',
        onCreate: (payload: any) => create.mutateAsync({ data: payload }),
        onUpdate: (payload: any) => update.mutateAsync({ id: 'registry-id', data: payload }),
      });
      return (
        <>
          <button
            disabled={isPending}
            onClick={() =>
              void save({
                name: 'Private registry',
                configuration: { $type: 'DockerHub', userName: 'test-user', pat: 'rejected-token' },
              }).catch(() => setFailed(true))
            }>
            Save
          </button>
          <div data-testid="failed">{String(failed)}</div>
          <div data-testid="location">{location.pathname}</div>
        </>
      );
    }
    try {
      const view = renderCitadel(<Probe />, { route: '/registries/add' });
      await view.user.click(view.getByRole('button', { name: 'Save' }));
      await waitFor(() => expect(view.getByTestId('failed')).toHaveTextContent('true'));
      expect(success).not.toHaveBeenCalled();
      expect(view.getByTestId('location')).toHaveTextContent('/registries/add');
      expect(view.getByRole('button', { name: 'Save' })).toBeEnabled();
    } finally {
      success.mockRestore();
    }
  });
});
