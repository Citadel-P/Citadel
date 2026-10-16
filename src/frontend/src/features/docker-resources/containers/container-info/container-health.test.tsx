import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ContainerHealth } from './container-health';
import { ContainerDetailsView } from '../hooks/useContainerInfoGroup';

const { read } = vi.hoisted(() => ({ read: vi.fn() }));
vi.mock('@/lib/hooks', () => ({ useRead: read }));
vi.mock('../hooks/useContainerHealthHistory', () => ({
  useContainerHealthHistory: () => ({ entries: [], isConnected: true, isLoading: false }),
}));
const resource = {
  resourceId: 'persisted',
  id: 'docker-id',
  state: 'Running',
  capabilities: { canInspect: false },
} as ContainerDetailsView;
beforeEach(() => {
  read.mockReset();
  read.mockReturnValue({ data: { data: [] }, refetch: vi.fn(), isFetching: false });
});
describe('health details permissions and unavailable evidence', () => {
  it('keeps inspection disabled for a read-only viewer while showing retained history', () => {
    render(<ContainerHealth resource={resource} />);
    expect(read).toHaveBeenCalledWith(
      'inspectContainer',
      { id: 'persisted' },
      expect.objectContaining({ enabled: false }),
    );
    expect(read.mock.calls.every(([name]) => name === 'inspectContainer')).toBe(true);
    expect(screen.queryByRole('button', { name: /Refresh/ })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Probe details' }));
    expect(screen.getByText(/Inspect permission is required/)).toBeInTheDocument();
    expect(read.mock.calls.every(([, , options]) => options.enabled === false)).toBe(true);
    expect(screen.getByText('No observations in this time range.')).toBeInTheDocument();
  });
  it('explains unsupported metadata without inventing no check', () => {
    render(
      <ContainerHealth
        resource={{ ...resource, capabilities: { canInspect: true } as ContainerDetailsView['capabilities'] }}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Probe details' }));
    expect(screen.getByText(/did not provide health-check metadata/)).toBeInTheDocument();
    expect(screen.queryByText('No health check configured.')).not.toBeInTheDocument();
  });
  it('keeps probe output collapsed until requested and preserves the snapshot when reopened', () => {
    read.mockReturnValue({
      data: {
        data: {
          healthMetadataObserved: true,
          state: { health: { log: [{ exitCode: 1, output: 'Connection refused' }] } },
        },
      },
    });
    render(
      <ContainerHealth
        resource={{ ...resource, capabilities: { canInspect: true } as ContainerDetailsView['capabilities'] }}
      />,
    );
    const trigger = screen.getByRole('button', { name: 'Probe details' });
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('Connection refused')).not.toBeInTheDocument();
    expect(read).toHaveBeenLastCalledWith(
      'inspectContainer',
      { id: 'persisted' },
      expect.objectContaining({ enabled: false }),
    );
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Connection refused')).toBeInTheDocument();
    expect(read).toHaveBeenLastCalledWith(
      'inspectContainer',
      { id: 'persisted' },
      expect.objectContaining({ enabled: true, refetchOnWindowFocus: false, refetchOnReconnect: false }),
    );
    fireEvent.click(trigger);
    expect(screen.queryByText('Connection refused')).not.toBeInTheDocument();
    expect(read).toHaveBeenLastCalledWith(
      'inspectContainer',
      { id: 'persisted' },
      expect.objectContaining({ enabled: true }),
    );
    fireEvent.click(trigger);
    expect(screen.getByText('Connection refused')).toBeInTheDocument();
  });
});
