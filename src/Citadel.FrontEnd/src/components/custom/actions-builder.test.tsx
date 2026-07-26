import { render, screen } from '@testing-library/react';
import { RefreshCw } from 'lucide-react';
import { createActionsBuilder } from './actions-builder';

describe('createActionsBuilder', () => {
  it('uses an explicit command title instead of deriving it from the key', () => {
    const { info } = createActionsBuilder<{ name: string }>()
      .addAction({
        key: 'sync',
        title: 'Reconcile drift',
        type: 'command',
        icon: RefreshCw,
        useHandler: () => ({
          canExecute: true,
          run: vi.fn(),
        }),
      })
      .build();
    const Action = info.sync;

    render(<Action resource={{ name: 'demo' }} />);

    expect(screen.getByRole('button', { name: /reconcile drift/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^sync$/i })).not.toBeInTheDocument();
  });

  it('shows command progress in the resource action bar', () => {
    const { info } = createActionsBuilder<{ name: string }>()
      .addAction({
        key: 'checkUpdates',
        title: 'Check for updates',
        type: 'command',
        icon: RefreshCw,
        useHandler: () => ({
          canExecute: true,
          isPending: true,
          run: vi.fn(),
        }),
      })
      .build();
    const Action = info.checkUpdates;

    render(<Action resource={{ name: 'demo' }} />);

    const button = screen.getByRole('button', { name: /check for updates/i });
    expect(button).toBeDisabled();
    expect(button.querySelector('.animate-spin')).toBeInTheDocument();
  });
});
