import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import type { NetworkView } from '@/api/generated/api.types';
import { NetworkDetailsBody } from './network-details';

const network = {
  containers: {},
  usage: {
    entries: [{ relationship: 'Attachment', containerId: 'container-id', containerName: 'komodo-core-1' }],
  },
} as NetworkView;

it.each([
  ['Running', false, false, 'bg-green-500'],
  ['Paused', false, false, 'bg-orange-500'],
  ['Exited', false, false, 'bg-gray-500'],
  ['Running', true, false, 'bg-gray-400'],
  ['Running', false, true, 'bg-gray-400'],
  [undefined, false, false, 'bg-gray-400'],
])('shows container state %s with stale entry=%s and network=%s', (state, isStale, networkStale, color) => {
  const { container } = render(
    <MemoryRouter>
      <NetworkDetailsBody
        platformId="platform"
        network={{
          ...network,
          isStale: networkStale,
          usage: {
            ...network.usage!,
            entries: [{ ...network.usage!.entries[0], containerState: state, isStale }],
          },
        }}
      />
    </MemoryRouter>,
  );
  expect(container.querySelector('li .rounded-full')).toHaveClass(color);
});

it('keeps the container link and explore action independently usable', async () => {
  const explore = vi.fn();
  render(
    <MemoryRouter>
      <NetworkDetailsBody network={network} platformId="platform" onExplore={explore} />
    </MemoryRouter>,
  );
  expect(screen.getByRole('link', { name: 'komodo-core-1' })).toHaveAttribute(
    'href',
    '/platforms/platform/containers/container-id',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Explore connections' }));
  expect(explore).toHaveBeenCalledWith('container-id');
});
