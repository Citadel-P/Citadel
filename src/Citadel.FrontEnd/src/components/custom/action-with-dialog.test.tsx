import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Rocket } from 'lucide-react';
import { ActionWithDialog } from './action-with-dialog';

describe('ActionWithDialog', () => {
  it('keeps the standard triggered confirmation flow working', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    renderCitadel(<ActionWithDialog name="nginx" title="Deploy" icon={<Rocket />} onClick={onConfirm} />);

    await user.click(screen.getByRole('button', { name: 'Deploy' }));

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByRole('heading', { name: 'Confirm Deploy' })).toBeInTheDocument();
    const confirmButton = within(dialog).getByRole('button', { name: 'Deploy' });
    expect(confirmButton).toBeDisabled();

    await user.type(within(dialog).getByRole('textbox', { name: 'Enter nginx to confirm' }), 'nginx');
    await user.click(confirmButton);

    await waitFor(() => expect(onConfirm).toHaveBeenCalledOnce());
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });
});
