import { screen, within, waitFor } from '@testing-library/react';
import { ContainerStateStatus } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { MemoryUsageCell } from './memory-usage-cell';

const stats = {
  memoryActive: 100 * 1024 ** 2,
  memoryActiveCache: 40 * 1024 ** 2,
  memoryCache: 20 * 1024 ** 2,
  memoryLimit: 512 * 1024 ** 2,
};
const cell = (value = stats) => <MemoryUsageCell state={ContainerStateStatus.Running} stats={value} />;

it('shows the breakdown on hover and keeps the working set as the table value', async () => {
  const { user } = renderCitadel(cell());
  const trigger = screen.getByRole('button', { name: /Memory breakdown:/ });
  expect(trigger).toHaveTextContent('100 MB / 512 MB');
  expect(trigger).not.toHaveAttribute('title');
  await user.hover(trigger);
  const card = await screen.findByRole('dialog', { name: 'Memory breakdown' });
  expect(trigger).not.toHaveFocus();
  expect(within(card).getByText('Memory excluding file cache').nextElementSibling).toHaveTextContent('60 MB');
  expect(within(card).getByText('Active file cache').nextElementSibling).toHaveTextContent('40 MB');
  expect(within(card).getByText('Inactive file cache').nextElementSibling).toHaveTextContent('20 MB');
  await user.hover(card);
  expect(card).toBeVisible();
  await user.unhover(card);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});

it('supports keyboard entry, Escape and click pinning', async () => {
  const { user } = renderCitadel(cell());
  const trigger = screen.getByRole('button');
  await user.tab();
  expect(trigger).toHaveFocus();
  await user.keyboard('{ArrowDown}');
  expect(screen.getByRole('dialog')).toHaveFocus();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
  await user.click(trigger);
  await user.unhover(trigger);
  expect(screen.getByRole('dialog')).toBeVisible();
  await user.click(trigger);
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it.each([null, undefined, -1, 200 * 1024 ** 2])('does not invent a breakdown for active cache %s', async (active) => {
  const { user } = renderCitadel(
    <MemoryUsageCell state={ContainerStateStatus.Running} stats={{ ...stats, memoryActiveCache: active }} />,
  );
  await user.click(screen.getByRole('button'));
  const card = screen.getByRole('dialog');
  expect(within(card).getByText('Memory excluding file cache').nextElementSibling).toHaveTextContent('—');
  expect(within(card).getByText('Active file cache').nextElementSibling).toHaveTextContent('—');
  expect(within(card).getByText('Inactive file cache').nextElementSibling).toHaveTextContent('20 MB');
});

it('does not present stale readings for stopped containers', () => {
  renderCitadel(<MemoryUsageCell state={ContainerStateStatus.Exited} stats={stats} />);
  expect(screen.getByText('0B / 0B')).toBeVisible();
  expect(screen.queryByRole('button')).not.toBeInTheDocument();
});
