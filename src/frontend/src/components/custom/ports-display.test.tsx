import { fireEvent, render, screen } from '@testing-library/react';
import { HostPortBinding } from '@/api/generated/api.types';
import { PortsDisplay } from './ports-display';

it('renders published ports from the Citadel contract and opens the port link', () => {
  const open = vi.spyOn(window, 'open').mockReturnValue(null);
  render(
    <PortsDisplay
      ports={{
        '80/tcp': [
          { hostIP: '0.0.0.0', hostPort: '8080' },
          { hostIP: '::', hostPort: '8080' },
        ],
        '443/tcp': [],
        '53/udp': null,
      }}
    />,
  );
  expect(screen.getAllByRole('button', { name: '8080' })).toHaveLength(1);
  fireEvent.click(screen.getByRole('button', { name: '8080' }));
  expect(open).toHaveBeenCalledWith('http://localhost:8080', '_blank');
  open.mockRestore();
});

it.each([null, {}, { '80/tcp': [] }, { '80/tcp': null }, [{ PrivatePort: 80, Type: 'tcp', IP: '' }]])(
  'does not crash or invent published ports for absent, exposed-only, or incompatible data: %j',
  (ports) => {
    const { container } = render(<PortsDisplay ports={ports as unknown as Record<string, HostPortBinding[]> | null} />);
    expect(container).toBeEmptyDOMElement();
  },
);

it('ignores malformed bindings without hiding valid published ports', () => {
  render(
    <PortsDisplay
      ports={
        { '80/tcp': [null, { hostPort: 80 }, { hostIP: '127.0.0.1', hostPort: '8080' }] } as unknown as Record<
          string,
          HostPortBinding[]
        >
      }
    />,
  );
  expect(screen.getByRole('button', { name: '8080' })).toBeVisible();
});
