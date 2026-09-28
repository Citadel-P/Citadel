import { PlatformConnectorType, PlatformType } from '@/api/generated/api.types';
import { createPlatform } from '@/test/factories/resources';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { createDefaultPlatformInput, usePlatformForm } from './usePlatformForm';

describe('Platform creation form', () => {
  it('uses the compatible createPlatform operation and returns to the Platform list', async () => {
    let requestBody: unknown;
    server.use(
      http.post('http://localhost/api/v1/platforms', async ({ request }) => {
        requestBody = await request.json();
        return HttpResponse.json({
          id: '019fffff-0000-7000-8000-000000000001',
          name: 'local-docker',
          address: 'http://localhost.docker',
          type: PlatformType.Docker,
          connectorType: PlatformConnectorType.Local,
        });
      }),
    );

    const { user } = renderCitadel(
      <>
        <CreatePlatformHarness />
        <CurrentPath />
      </>,
      { route: '/platforms/add' },
    );

    await user.click(screen.getByRole('button', { name: 'Create fixture Platform' }));

    await waitFor(() => expect(screen.getByTestId('current-path')).toHaveTextContent('/platforms'));
    expect(requestBody).toEqual({
      name: 'local-docker',
      address: null,
      description: null,
      type: PlatformType.Docker,
      connectorType: PlatformConnectorType.Local,
      pruneHistoricalSwarmTaskContainers: true,
      tagIds: [],
    });
  });

  it('preserves the current address when editing a local Platform when the submitted address is null', async () => {
    const platform = createPlatform({ id: 'local-platform', address: 'http://localhost.docker' });
    let requestBody: unknown;
    server.use(
      http.patch(`http://localhost/api/v1/platforms/${platform.id}`, async ({ request }) => {
        requestBody = await request.json();
        return HttpResponse.json(platform);
      }),
    );

    function EditPlatformHarness() {
      const { save } = usePlatformForm('edit', platform);
      return (
        <button onClick={() => save({ name: 'renamed', address: null, connectorType: PlatformConnectorType.Local })}>
          Save Platform
        </button>
      );
    }

    const { user } = renderCitadel(<EditPlatformHarness />);
    await user.click(screen.getByRole('button', { name: 'Save Platform' }));
    await waitFor(() => expect(requestBody).toMatchObject({ name: 'renamed', address: platform.address }));
  });
});

const CreatePlatformHarness = () => {
  const { save } = usePlatformForm();
  return (
    <button type="button" onClick={() => save({ ...createDefaultPlatformInput(), name: 'local-docker' })}>
      Create fixture Platform
    </button>
  );
};

const CurrentPath = () => <span data-testid="current-path">{useLocation().pathname}</span>;
