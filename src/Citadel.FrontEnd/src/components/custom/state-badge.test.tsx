import { render, screen } from '@testing-library/react';
import { StateBadge } from './state-badge';

describe('StateBadge', () => {
  it('uses resource-specific state semantics', () => {
    const { rerender } = render(<StateBadge value="Running" kind="swarmTask" />);
    expect(screen.getByText('Running')).toHaveClass('text-green-700');

    rerender(<StateBadge value="Running" kind="run" />);
    expect(screen.getByText('Running')).toHaveClass('text-blue-700');
  });

  it('allows a display label without changing state mapping', () => {
    render(<StateBadge value="ApplyingRetention" kind="run" label="Applying retention" />);
    expect(screen.getByText('Applying retention')).toHaveClass('text-blue-700');
  });

  it('supports boolean resource states without treating false as missing', () => {
    render(<StateBadge value={false} />);
    expect(screen.getByText('false')).toHaveClass('text-muted-foreground');
  });

  it('shows a paused state as a warning', () => {
    render(<StateBadge value="Paused" />);
    expect(screen.getByText('Paused')).toHaveClass('text-orange-500');
  });

  it.each([
    ['activity', 'Information', 'Info', 'text-blue-700'],
    ['activity', 'Failure', 'Failure', 'text-red-700'],
    ['alertSeverity', 'Critical', 'Critical', 'text-red-700'],
    ['alertEvent', 'Acknowledged', 'Acknowledged', 'text-orange-600'],
    ['alertEvent', 'Resolved', 'Resolved', 'text-green-700'],
  ] as const)('maps %s value %s to its existing label and color', (kind, value, label, className) => {
    render(<StateBadge value={value} kind={kind} />);
    expect(screen.getByText(label)).toHaveClass(className);
  });
});
