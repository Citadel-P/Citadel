import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

  it('shows the disabled reason in a tooltip', async () => {
    const user = userEvent.setup();
    const { info } = createActionsBuilder<{ name: string }>()
      .addAction({
        key: 'checkUpdates',
        title: 'Check for updates',
        type: 'command',
        icon: RefreshCw,
        useHandler: () => ({
          canExecute: false,
          disabledReason: 'Update checks are only available for external tagged images.',
          run: vi.fn(),
        }),
      })
      .build();
    const Action = info.checkUpdates;

    render(<Action resource={{ name: 'demo' }} />);

    await user.hover(screen.getByRole('button', { name: /check for updates/i }).parentElement!);

    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      'Update checks are only available for external tagged images.',
    );
  });

  it('renders a disabled grouped toggle action with its reason', async () => {
    const user = userEvent.setup();
    const run = vi.fn();
    const { group } = createActionsBuilder<{ name: string }>()
      .addAction({
        key: 'lifecycle',
        type: 'toggle',
        predicate: () => false,
        primary: {
          title: 'Start',
          icon: RefreshCw,
          useHandler: () => ({
            canExecute: false,
            disabledReason: 'The selected container cannot be started yet.',
            run,
          }),
        },
        secondary: {
          title: 'Stop',
          icon: RefreshCw,
          useHandler: () => ({ canExecute: true, run }),
        },
      })
      .build();
    const Action = group.lifecycle;

    render(<Action resources={[{ name: 'demo' }]} />);

    const button = screen.getByRole('button', { name: /start/i });
    expect(button).toBeDisabled();
    await user.hover(button.parentElement!);

    expect(await screen.findByRole('tooltip')).toHaveTextContent('The selected container cannot be started yet.');
  });
});
