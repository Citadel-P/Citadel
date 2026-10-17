import { screen, waitFor } from '@testing-library/react';
import { useLocation, useNavigate } from 'react-router';
import { AutoUpdateStatus } from '@/api/generated/api.types';
import { DeploymentComponents } from '@/features/deployments';
import { StackComponents } from '@/features/stacks';
import { PlatformComponents } from '@/features/platforms';
import { SwarmServiceComponents } from '@/features/swarm-services';
import { renderCitadel } from '@/test/render-citadel';
import { RegularResourceView } from './regular-resource';
import type { RegularResourceComponents } from './types';

vi.hoisted(() => {
  document.queryCommandSupported ??= () => false;
});
const data = vi.hoisted(() => ({ items: [] as Record<string, unknown>[] }));
vi.mock('@/features/deployments/hooks/useDeploymentsGroup', () => ({
  useDeploymentsGroup: () => ({ deployments: data.items, isLoading: false }),
}));
vi.mock('@/features/stacks/hooks/useStacksGroup', () => ({
  useStacksGroup: () => ({ stacks: data.items, isLoading: false }),
}));
vi.mock('@/features/platforms/hooks/usePlatformsGroup', () => ({
  usePlatformsGroup: () => ({ platformsMessage: data.items, isLoading: false }),
}));
vi.mock('@/features/swarm-services/hooks/useSwarmServicesGroup', () => ({
  useSwarmServicesGroup: () => ({ services: data.items, isLoading: false }),
}));
const makeItem = (name: string, update: boolean) => ({
  id: name,
  name,
  dockerName: name,
  autoUpdateState: { status: update ? AutoUpdateStatus.UpdateAvailable : AutoUpdateStatus.UpToDate },
  stackUpdateState: { recreateStackOnNewImageState: { autoUpdateStates: [{ updateAvailable: update }] } },
});
beforeEach(() => {
  data.items = [makeItem('api-one', true), makeItem('api-two', false)];
});
function Location() {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <output aria-label="Location">{location.search}</output>
      <button onClick={() => navigate(-1)}>Back</button>
    </>
  );
}
function View({ config, type }: { config: RegularResourceComponents; type: string }) {
  return (
    <>
      <RegularResourceView
        type={type}
        showTaskSheet={false}
        Components={{
          ...config,
          header: { ...config.header, showTagFilter: false, showPlatformFilter: false, showAdd: false },
          GroupActions: undefined,
          Content: ({ items }) => (
            <ul>
              {items.map((item: { id: string; name: string }) => (
                <li key={item.id}>{item.name}</li>
              ))}
            </ul>
          ),
        }}
      />
      <Location />
    </>
  );
}

it.each([
  ['Deployment', DeploymentComponents],
  ['Stack', StackComponents],
  ['Platform', PlatformComponents],
  ['SwarmService', SwarmServiceComponents],
] as const)('%s lists ignore retired overview filters', (type, config) => {
  renderCitadel(<View type={type} config={config} />, { route: '/?overview=stopped' });
  expect(screen.getByText('api-one')).toBeVisible();
  expect(screen.getByText('api-two')).toBeVisible();
  expect(screen.queryByText('Filter the current view')).not.toBeInTheDocument();
  expect(screen.queryByRole('region', { name: /overview/i })).not.toBeInTheDocument();
});

it.each([
  ['Deployment', DeploymentComponents],
  ['Stack', StackComponents],
] as const)(
  '%s keeps updates filtering, search, history and realtime changes working without cards',
  async (type, config) => {
    const { user, rerender } = renderCitadel(<View type={type} config={config} />, {
      route: '/?updates=available&tags=prod',
    });
    expect(screen.getByText('api-one')).toBeVisible();
    expect(screen.queryByText('api-two')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Updates available' }));
    expect(screen.getByText('api-two')).toBeVisible();
    expect(screen.getByLabelText('Location')).toHaveTextContent('?tags=prod');
    await user.click(screen.getByRole('button', { name: 'Back' }));
    expect(screen.queryByText('api-two')).not.toBeInTheDocument();
    data.items = [makeItem('api-one', false), makeItem('api-two', true)];
    rerender(<View type={type} config={config} />);
    expect(screen.queryByText('api-one')).not.toBeInTheDocument();
    expect(screen.getByText('api-two')).toBeVisible();
    await user.type(screen.getByPlaceholderText(`Search ${type.toLowerCase()}s…`), 'one');
    await waitFor(() => expect(screen.queryByText('api-two')).not.toBeInTheDocument());
    await user.click(screen.getByRole('button', { name: 'Updates available' }));
    expect(screen.getByText('api-one')).toBeVisible();
    expect(screen.queryByText('api-two')).not.toBeInTheDocument();
  },
);
