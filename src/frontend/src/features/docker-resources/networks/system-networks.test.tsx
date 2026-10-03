import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { NetworkView } from '@/api/generated/api.types';
import { NetworkNameRow } from './table';
import { NetworkInfoComponents } from './network-info';

const network = (isSystem: boolean) =>
  ({
    id: 'network-id',
    name: isSystem ? 'bridge' : 'application-network',
    inUse: false,
    isSystem,
  }) as NetworkView;

describe('system networks', () => {
  it('shows a System badge for a Docker system network', () => {
    render(
      <MemoryRouter>
        <NetworkNameRow network={network(true)} />
      </MemoryRouter>,
    );

    expect(screen.getByLabelText('Docker system network')).toBeVisible();
  });

  it('does not show a System badge for an application network', () => {
    render(
      <MemoryRouter>
        <NetworkNameRow network={network(false)} />
      </MemoryRouter>,
    );

    expect(screen.queryByLabelText('Docker system network')).not.toBeInTheDocument();
  });

  it('shows the System badge in the network detail header', () => {
    const NameSuffix = NetworkInfoComponents.Header.NameSuffix!;

    render(<NameSuffix resource={{ ...network(true), containers: {}, peers: [] } as NetworkView} />);

    expect(screen.getByLabelText('Docker system network')).toBeVisible();
  });
});
