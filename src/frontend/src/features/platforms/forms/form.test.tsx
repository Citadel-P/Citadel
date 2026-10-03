import { PlatformConnectorType, PlatformType } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { vi } from 'vitest';
import { PlatformForm } from './form';
import { PlatformFormInput } from './hooks/usePlatformForm';

vi.mock('@/components/custom/form-builder', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/components/custom/form-builder')>()),
  FormShell: ({
    original,
    update,
    onSave,
  }: {
    original: PlatformFormInput;
    update: Partial<PlatformFormInput>;
    onSave: (input: PlatformFormInput) => Promise<void>;
  }) => (
    <>
      <span data-testid="default-connector">{original.connectorType}</span>
      <span data-testid="connector">{update.connectorType ?? original.connectorType}</span>
      <button onClick={() => onSave({ ...original, ...update, name: 'local-docker' })}>Save platform</button>
    </>
  ),
}));

const localPlatform = {
  id: '019fffff-0000-7000-8000-000000000001',
  name: 'local-docker',
  address: 'http://localhost.docker',
  type: PlatformType.Docker,
  connectorType: PlatformConnectorType.Local,
};
const CurrentPath = () => <span data-testid="current-path">{useLocation().pathname}</span>;

describe('Platform connector setup requests', () => {
  it('does not request Agent setup when Local creation updates the platform list', async () => {
    const agentSetup = vi.fn(() => HttpResponse.json({}));
    const createRequested = vi.fn();
    let platformCreated = false;
    let finishCreate!: () => void;
    const pendingCreate = new Promise<void>((resolve) => {
      finishCreate = resolve;
    });
    server.use(
      http.get('http://localhost/api/v1/platforms', () =>
        HttpResponse.json({ platforms: platformCreated ? [localPlatform] : [] }),
      ),
      http.get('http://localhost/api/v1/platforms/agent/setup', agentSetup),
      http.post('http://localhost/api/v1/platforms', async ({ request }) => {
        createRequested(await request.json());
        await pendingCreate;
        platformCreated = true;
        return HttpResponse.json(localPlatform);
      }),
    );
    const { user, queryClient } = renderCitadel(
      <>
        <PlatformForm mode="add" />
        <CurrentPath />
      </>,
      { route: '/platforms/add' },
    );
    try {
      await waitFor(() => expect(queryClient.getQueryState(['listPlatforms', {}])?.status).toBe('success'));
      await user.click(screen.getByRole('button', { name: 'Save platform' }));
      await waitFor(() =>
        expect(createRequested).toHaveBeenCalledWith(
          expect.objectContaining({ connectorType: PlatformConnectorType.Local }),
        ),
      );
      // A realtime notification can update the list before POST completes;
      // the post-save list invalidation can produce the same transition.
      act(() => queryClient.setQueryData(['listPlatforms', {}], { data: { platforms: [localPlatform] } }));
      await waitFor(() => expect(screen.getByTestId('default-connector')).toHaveTextContent('Agent'));
      expect(screen.getByTestId('connector')).toHaveTextContent('Local');
      expect(queryClient.getQueryState(['getAgentSetup', {}])?.fetchStatus).toBe('idle');
    } finally {
      finishCreate();
    }
    await waitFor(() => expect(screen.getByTestId('current-path')).toHaveTextContent(/^\/platforms$/));
    expect(agentSetup).not.toHaveBeenCalled();
  });

  it('still loads Agent setup when a Local platform already exists before opening the form', async () => {
    const agentSetup = vi.fn(() => HttpResponse.json({}));
    server.use(
      http.get('http://localhost/api/v1/platforms', () => HttpResponse.json({ platforms: [localPlatform] })),
      http.get('http://localhost/api/v1/platforms/agent/setup', agentSetup),
    );
    renderCitadel(<PlatformForm mode="add" />);
    await waitFor(() => expect(screen.getByTestId('connector')).toHaveTextContent('Agent'));
    await waitFor(() => expect(agentSetup).toHaveBeenCalledOnce());
  });
});
