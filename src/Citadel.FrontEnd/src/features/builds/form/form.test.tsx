import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import {
  AuthorizedProject,
  BuildProjectBuilderKind,
  PlatformConnectorType,
  ResourceControlState,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BuildForm } from './form';

vi.mock('@/lib/monaco', () => ({ MonacoEditor: () => null, MonacoDiff: () => null }));

const project = {
  id: '00000000-0000-0000-0000-000000000201',
  name: 'app-image',
  enabled: true,
  gitRepositoryId: 'repo-id',
  branch: 'main',
  contextPath: '.',
  dockerfilePath: 'Dockerfile',
  builderKind: BuildProjectBuilderKind.Platform,
  platformId: 'platform-id',
  pushToRegistry: true,
  registryId: null,
  imageRepository: '',
  tagTemplates: ['latest'],
  buildArgs: [],
  buildSecrets: [],
  timeoutSeconds: 1800,
  retentionRunCount: 20,
  rowVersion: 1,
  capabilities: { canRead: true, canWrite: true, canExecute: true },
  normalizedName: 'APP-IMAGE',
  description: null,
  target: null,
  buildAgentPoolId: null,
  webhook: null,
  currentRunId: null,
  controlState: ResourceControlState.Idle,
  controlStartedAt: null,
  createdByActorId: 'actor-id',
  createdAt: '2026-10-03T10:00:00Z',
  updatedAt: '2026-10-03T10:00:00Z',
  archivedAt: null,
  tags: [],
  latestRun: null,
} satisfies AuthorizedProject;

beforeEach(() => {
  server.use(
    http.get('http://localhost/api/v1/*', ({ request }) => {
      const path = new URL(request.url).pathname;
      if (path.endsWith('/platforms'))
        return HttpResponse.json({
          platforms: [{ id: 'platform-id', name: 'Builder', connectorType: PlatformConnectorType.Local }],
        });
      return HttpResponse.json({ capabilities: [], pools: [], secrets: [], refs: [], branches: [], resources: [] });
    }),
  );
});

it('defaults to pushing and hides the required registry fields when disabled', async () => {
  const { user } = renderCitadel(<BuildForm mode="add" />, { route: '/builds/add' });
  const toggle = screen.getByRole('switch', { name: 'Push to registry' });
  expect(toggle).toBeChecked();
  expect(screen.getByText('Image repository')).toBeVisible();
  await user.click(toggle);
  expect(toggle).not.toBeChecked();
  expect(screen.queryByText('Image repository')).not.toBeInTheDocument();
  expect(screen.queryByText('Select registry')).not.toBeInTheDocument();
  expect(screen.getByText(/The image is tagged locally/)).toBeVisible();
  await user.click(toggle);
  expect(screen.getByText('Image repository')).toBeVisible();
});

it('saves a local-only build without registry or image repository', async () => {
  let saved: unknown;
  server.use(
    http.patch(`http://localhost/api/v1/buildProjects/${project.id}`, async ({ request }) => {
      saved = await request.json();
      return HttpResponse.json({ ...project, pushToRegistry: false });
    }),
  );
  const { user } = renderCitadel(
    <Routes>
      <Route path="/builds/edit/:id" element={<BuildForm mode="edit" resource={project} />} />
    </Routes>,
    { route: `/builds/edit/${project.id}` },
  );
  await user.click(screen.getByRole('switch', { name: 'Push to registry' }));
  await user.click(screen.getAllByRole('button', { name: /^Save$/ })[0]);
  await waitFor(() => expect(saved).toMatchObject({ pushToRegistry: false, registryId: null, imageRepository: '' }));
});
