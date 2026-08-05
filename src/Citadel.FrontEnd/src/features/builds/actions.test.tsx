import type { BuildProjectView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { BuildInfoActions } from './actions';

const buildProjectId = '00000000-0000-0000-0000-000000000201';

const LocationProbe = () => {
  const location = useLocation();
  return <span data-testid="location">{location.pathname}</span>;
};

describe('BuildInfoActions', () => {
  it('returns to the build list after archiving the open build', async () => {
    server.use(
      http.delete(`http://localhost/api/v1/buildProjects/${buildProjectId}`, () =>
        HttpResponse.json({}, { status: 200 }),
      ),
    );
    const DeleteAction = BuildInfoActions.delete;
    const project = { id: buildProjectId, name: 'Application image' } as BuildProjectView;
    const { user } = renderCitadel(
      <>
        <DeleteAction resource={project} />
        <LocationProbe />
      </>,
      { route: `/builds/edit/${buildProjectId}` },
    );

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = await screen.findByRole('dialog');
    await user.type(within(dialog).getByRole('textbox'), project.name);
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));

    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/builds'));
  });
});
