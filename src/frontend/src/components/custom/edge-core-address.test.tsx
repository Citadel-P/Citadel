import { render, screen } from '@testing-library/react';
import { EdgeCoreAddress } from './edge-core-address';

const commandFor = (coreUrl: string, token = 'enrollment-token') =>
  `docker run -d \\\n  -e 'CITADEL_CORE_URL=${coreUrl}' \\\n  -e 'CITADEL_EDGE_ENROLLMENT_TOKEN=${token}' \\\n  'citadel-agent:local'`;
const setup = (coreUrl: string, token?: string) => (
  <EdgeCoreAddress coreUrl={coreUrl} dockerCommand={commandFor(coreUrl, token)}>
    {(command) => <pre data-testid="docker-command">{command}</pre>}
  </EdgeCoreAddress>
);

describe('Edge Core address', () => {
  it.each([
    'http://host.docker.internal:8001',
    'http://localhost:8001',
    'http://127.0.0.1:18001',
    'https://[::1]:8443',
  ])('shows a placeholder command without an input for %s', (coreUrl) => {
    render(setup(coreUrl));
    const url = new URL(coreUrl);
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
    expect(screen.queryByText('Core address')).not.toBeInTheDocument();
    expect(screen.getByTestId('docker-command').textContent).toBe(
      commandFor(`${url.protocol}//core-ip-or-hostname:${url.port}`),
    );
  });

  it('keeps the configured public address', () => {
    render(setup('https://edge.example.com'));
    expect(screen.getByTestId('docker-command').textContent).toBe(commandFor('https://edge.example.com'));
  });

  it('updates the command after token rotation', () => {
    const { rerender } = render(setup('http://localhost:8001'));
    rerender(setup('http://localhost:8001', 'rotated-token'));
    expect(screen.getByTestId('docker-command').textContent).toBe(
      commandFor('http://core-ip-or-hostname:8001', 'rotated-token'),
    );
  });

  it('uses a placeholder for an invalid configured address', () => {
    render(setup('invalid-address'));
    expect(screen.getByTestId('docker-command').textContent).toBe(commandFor('http://core-ip-or-hostname'));
  });

  it('requests new instructions if the Core assignment cannot be replaced', () => {
    render(
      <EdgeCoreAddress coreUrl="http://localhost:8001" dockerCommand="docker run citadel-agent:local">
        {(command) => <pre data-testid="docker-command">{command}</pre>}
      </EdgeCoreAddress>,
    );
    expect(screen.queryByTestId('docker-command')).not.toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('Generate a new enrollment command');
  });
});
