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

  it('omits actions that are not available for the current resources', () => {
    const { info, group } = createActionsBuilder<{ name: string; supportsAction: boolean }>()
      .addAction({
        key: 'restart',
        title: 'Restart',
        type: 'command',
        icon: RefreshCw,
        isVisible: (resources) => {
          const items = Array.isArray(resources) ? resources : [resources];
          return items.every((resource) => resource.supportsAction);
        },
        useHandler: () => ({
          canExecute: true,
          run: vi.fn(),
        }),
      })
      .build();
    const InfoAction = info.restart;
    const GroupAction = group.restart;

    const { rerender } = render(
      <>
        <InfoAction resource={{ name: 'unsupported', supportsAction: false }} />
        <GroupAction resources={[{ name: 'unsupported', supportsAction: false }]} />
      </>,
    );

    expect(screen.queryByRole('button', { name: /restart/i })).not.toBeInTheDocument();

    rerender(
      <>
        <InfoAction resource={{ name: 'supported', supportsAction: true }} />
        <GroupAction resources={[{ name: 'supported', supportsAction: true }]} />
      </>,
    );

    expect(screen.getAllByRole('button', { name: /restart/i })).toHaveLength(2);
  });
});

describe('action failure recovery', () => {
  it('reports custom action network failures', async () => {
    const { toast } = await import('sonner');
    const notice = vi.spyOn(toast, 'error');
    const { info } = createActionsBuilder<{ name: string }>()
      .addAction({
        key: 'sync',
        type: 'command',
        icon: RefreshCw,
        useHandler: () => ({
          run: async () => {
            throw new TypeError('Failed to fetch');
          },
        }),
      })
      .build();
    const Action = info.sync;
    render(<Action resource={{ name: 'demo' }} />);
    await userEvent.setup().click(screen.getByRole('button', { name: 'Sync' }));
    expect(notice).toHaveBeenCalledWith(
      'Request failed',
      expect.objectContaining({ description: expect.stringContaining('Check your connection') }),
    );
  });

  it('keeps a failed confirmation open and allows retry', async () => {
    const { toast } = await import('sonner');
    const notice = vi.spyOn(toast, 'error');
    const run = vi
      .fn()
      .mockRejectedValueOnce({ status: 409, error: { detail: 'Resource is busy.' } })
      .mockResolvedValue(undefined);
    const { info } = createActionsBuilder<{ name: string }>()
      .addAction({ key: 'sync', type: 'command', icon: RefreshCw, confirm: true, useHandler: () => ({ run }) })
      .build();
    const Action = info.sync;
    const user = userEvent.setup();
    render(<Action resource={{ name: 'demo' }} />);
    await user.click(screen.getByRole('button', { name: 'Sync' }));
    await user.type(screen.getByRole('textbox'), 'demo');
    const { within, waitFor } = await import('@testing-library/react');
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Sync' }));
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(notice).toHaveBeenCalledWith('Request failed', { description: 'Resource is busy.' });
    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Sync' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(run).toHaveBeenCalledTimes(2);
  });
});
