import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router';
import type { NetworkView } from '@/api/generated/api.types';
import { TabsWithActions, TabActionsSlot } from '@/components/custom/tab-actions';
import { TabsList, TabsTrigger } from '@/components/ui/tabs';
import { NetworkMapAction } from './network-map-action';

const inspection = vi.hoisted(() => vi.fn(() => ({ isLoading: false })));
vi.mock('../hooks/useNetworkInspection', () => ({ useNetworkInspection: inspection }));
vi.mock('../map/canvas', () => ({ default: () => <div>Network diagram</div> }));

const network = {
  id: 'network-id',
  name: 'backend',
  driver: 'bridge',
  scope: 'local',
  dockerNodeId: 'node-a',
  capabilities: { canRead: true, canInspect: true, canWrite: false, canExecute: false },
  containers: {},
} as NetworkView;

function renderAction(resource = network) {
  return render(
    <MemoryRouter initialEntries={['/platforms/platform-a/networks/network-id/?dockerNodeId=node-a']}>
      <Routes>
        <Route
          path="/platforms/:platformId/networks/:resourceId"
          element={
            <TabsWithActions defaultValue="Inspect">
              <div>
                <TabsList>
                  <TabsTrigger value="Inspect">Inspect</TabsTrigger>
                </TabsList>
                <TabActionsSlot />
              </div>
              <NetworkMapAction network={resource} />
            </TabsWithActions>
          }
        />
      </Routes>
    </MemoryRouter>,
  );
}

it('opens the scoped sheet from the tab action and restores focus on close', async () => {
  renderAction();
  const user = userEvent.setup();
  const trigger = screen.getByRole('button', { name: 'View connections' });
  expect(screen.getByRole('tablist')).not.toContainElement(trigger);
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await user.click(trigger);
  expect(await screen.findByText('Network diagram')).toBeVisible();
  expect(inspection).toHaveBeenCalledWith(network, 'platform-a');
  expect(screen.queryByRole('link', { name: 'Open network details' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Map options' }));
  expect(screen.queryByRole('menuitem', { name: /Explore workload/ })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Maximize network sheet' }));
  expect(screen.getByRole('button', { name: 'Restore network sheet' })).toBeVisible();
  await user.keyboard('{Escape}');
  expect(screen.getByRole('dialog')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Maximize network sheet' })).toBeVisible();
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  await waitFor(() => expect(trigger).toHaveFocus());
});

it('does not offer the map without inspection permission', () => {
  renderAction({ ...network, capabilities: { ...network.capabilities!, canInspect: false } });
  expect(screen.queryByRole('button', { name: 'View connections' })).not.toBeInTheDocument();
});
