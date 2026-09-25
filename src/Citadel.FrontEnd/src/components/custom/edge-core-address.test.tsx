import { fireEvent, render, screen } from '@testing-library/react';
import { EdgeCoreAddress } from './edge-core-address';

const commandFor = (coreUrl: string, token = 'enrollment-token') =>
  `docker run -d \\\n  -e 'CITADEL_CORE_URL=${coreUrl}' \\\n  -e 'CITADEL_EDGE_ENROLLMENT_TOKEN=${token}' \\\n  'citadel-agent:local'`;
const setup = (coreUrl: string, token?: string) => (
  <EdgeCoreAddress coreUrl={coreUrl} dockerCommand={commandFor(coreUrl, token)}>
    {(command) => <pre data-testid="docker-command">{command}</pre>}
  </EdgeCoreAddress>
);

describe('Edge Core address', () => {
  it.each(['http://host.docker.internal:8001', 'http://localhost:8001', 'http://127.0.0.1:18001', 'http://[::1]:8001'])(
    'asks for the Core host instead of offering a command with %s',
    (coreUrl) => {
      render(setup(coreUrl));
      expect(screen.getByLabelText('Core address')).toHaveValue('');
      expect(screen.queryByTestId('docker-command')).not.toBeInTheDocument();

      fireEvent.change(screen.getByLabelText('Core address'), { target: { value: 'http://192.168.1.20:8001' } });
      expect(screen.getByTestId('docker-command').textContent).toBe(commandFor('http://192.168.1.20:8001'));
    },
  );

  it('prefills the configured public address and updates the command when edited', () => {
    render(setup('https://edge.example.com'));
    expect(screen.getByLabelText('Core address')).toHaveValue('https://edge.example.com');
    expect(screen.getByTestId('docker-command').textContent).toBe(commandFor('https://edge.example.com'));

    fireEvent.change(screen.getByLabelText('Core address'), { target: { value: 'https://core.internal:8443' } });
    expect(screen.getByTestId('docker-command').textContent).toBe(commandFor('https://core.internal:8443'));
  });

  it('uses the new token after rotation while keeping the chosen address', () => {
    const { rerender } = render(setup('http://localhost:8001'));
    fireEvent.change(screen.getByLabelText('Core address'), { target: { value: 'https://core.example.com' } });
    rerender(setup('http://localhost:8001', 'rotated-token'));
    expect(screen.getByTestId('docker-command').textContent).toBe(
      commandFor('https://core.example.com', 'rotated-token'),
    );
  });

  it.each([
    'core-host',
    'ftp://core-host',
    'https://user:password@core-host',
    'https://core-host/path',
    'https://core-host?key=value',
    'https://core-host#fragment',
  ])('hides the command for an invalid Core address: %s', (value) => {
    render(setup('https://core.example.com'));
    fireEvent.change(screen.getByLabelText('Core address'), { target: { value } });
    expect(screen.queryByTestId('docker-command')).not.toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('Enter an HTTP or HTTPS address');
  });

  it('allows an explicit Docker Desktop address for an Agent on the same host', () => {
    render(setup('http://localhost:8001'));
    fireEvent.change(screen.getByLabelText('Core address'), { target: { value: 'http://host.docker.internal:8001' } });
    expect(screen.getByTestId('docker-command').textContent).toBe(commandFor('http://host.docker.internal:8001'));
  });
});
