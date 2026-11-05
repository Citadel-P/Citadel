import { render, screen } from '@testing-library/react';
import { PasswordFeedback } from './password-feedback';

const feedback = (password: string, minimumLength = 15, maximumLength = 128) => (
  <PasswordFeedback password={password} minimumLength={minimumLength} maximumLength={maximumLength} />
);

it('shows the configured policy quietly before typing', () => {
  render(feedback('', 8));
  expect(screen.getByText('Use 8 to 128 characters.')).toBeVisible();
  expect(screen.getByText('0 characters')).toBeVisible();
  expect(screen.queryByRole('meter')).not.toBeInTheDocument();
});

it('counts Unicode code points like server validation and does not score overlong input', () => {
  const { rerender } = render(feedback('a😀é', 2, 3));
  expect(screen.getByText('3 characters · min 2')).toBeVisible();
  rerender(feedback('a😀éb', 2, 3));
  expect(screen.getByText('4 characters · min 2')).toBeVisible();
  expect(screen.getByText('Maximum 3 characters')).toBeVisible();
  expect(screen.queryByRole('meter')).not.toBeInTheDocument();
});

it('updates strength immediately while typing and clears it with the password', async () => {
  const { rerender } = render(feedback('passwordpasswordpassword'));
  const meter = await screen.findByRole('meter', { name: 'Estimated password strength' });
  expect(meter).toHaveAttribute('aria-valuetext', 'Weak');
  rerender(feedback('marigold fjord lantern cobalt'));
  expect(screen.getByRole('meter')).toHaveAttribute('aria-valuetext', 'Strong');
  rerender(feedback(''));
  expect(screen.queryByRole('meter')).not.toBeInTheDocument();
  expect(screen.getByText('0 characters')).toBeVisible();
});

it('does not mistake predictable symbols and numbers for strength', async () => {
  render(feedback('P@ssw0rd123!'));
  expect(await screen.findByRole('meter')).toHaveAttribute('aria-valuetext', 'Weak');
});

it.each(['azazazazazazazazazazazaazaz', 'azazazazazazazazazazazaza'])(
  'shows Weak for the reported repetitive password: %s',
  async (password) => {
    render(feedback(password));
    expect(await screen.findByRole('meter')).toHaveAttribute('aria-valuetext', 'Weak');
  },
);

it('reserves Strong for the highest estimate', async () => {
  render(feedback('tulip window'));
  expect(await screen.findByRole('meter')).toHaveAttribute('aria-valuetext', 'Fair');
});
