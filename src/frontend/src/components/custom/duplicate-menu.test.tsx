import { screen } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { DuplicateMenu } from './duplicate-menu';

it.each([
  ['Configuration only', 'configuration'],
  ['Configuration + data', 'data'],
] as const)('offers both modes and selects %s explicitly', async (label, mode) => {
  const onSelect = vi.fn();
  const { user } = renderCitadel(<DuplicateMenu onSelect={onSelect} />);
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  expect(onSelect).not.toHaveBeenCalled();
  expect(screen.getAllByRole('menuitem')).toHaveLength(2);
  await user.click(screen.getByRole('menuitem', { name: new RegExp(label.replace('+', '\\+')) }));
  expect(onSelect).toHaveBeenCalledExactlyOnceWith(mode);
});

it('does not open when duplication is unavailable', async () => {
  const { user } = renderCitadel(<DuplicateMenu onSelect={vi.fn()} disabled />);
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

it('shows the disabled reason inline and prevents keyboard selection of data copying', async () => {
  const onSelect = vi.fn();
  const { user } = renderCitadel(<DuplicateMenu onSelect={onSelect} dataDisabledReason="No volumes configured." />);
  screen.getByRole('button', { name: 'Duplicate' }).focus();
  await user.keyboard('{Enter}');
  expect(screen.getByText('No volumes configured.')).toBeVisible();
  expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).toHaveAttribute('aria-disabled', 'true');
  await user.keyboard('{End}{Enter}');
  expect(onSelect).toHaveBeenCalledExactlyOnceWith('configuration');
});
