import { render, screen, within } from '@testing-library/react';
import type { ButtonGroupComponent } from '@/pages/types';
import { SwarmServiceComponents } from '.';

vi.mock('@/components/custom/action-bar', () => ({
  ActionBar: ({
    actions,
    standaloneActions = [],
  }: {
    actions: ButtonGroupComponent<unknown>[];
    standaloneActions?: ButtonGroupComponent<unknown>[];
  }) => (
    <>
      <div data-testid="standalone-actions">
        {standaloneActions.map((Action, index) => (
          <Action resources={[]} key={index} />
        ))}
      </div>
      <div data-testid="grouped-actions">
        {actions.map((Action, index) => (
          <Action resources={[]} key={index} />
        ))}
      </div>
    </>
  ),
}));

vi.mock('./actions', () => ({
  SwarmServiceDropdownActions: {},
  SwarmServiceGroupActions: {
    apply: () => <button type="button">Apply</button>,
    duplicate: () => <button type="button">Duplicate</button>,
    checkUpdates: () => <button type="button">Check for Updates</button>,
    delete: () => <button type="button">Delete</button>,
  },
}));

describe('SwarmServiceComponents', () => {
  it('keeps duplicate and update checks outside the operational action group', () => {
    const GroupActions = SwarmServiceComponents.GroupActions!;

    render(<GroupActions items={[]} />);

    const standalone = within(screen.getByTestId('standalone-actions'));
    const grouped = within(screen.getByTestId('grouped-actions'));

    expect(standalone.getByRole('button', { name: 'Duplicate' })).toBeVisible();
    expect(standalone.getByRole('button', { name: 'Check for Updates' })).toBeVisible();
    expect(grouped.queryByRole('button', { name: 'Duplicate' })).not.toBeInTheDocument();
    expect(grouped.queryByRole('button', { name: 'Check for Updates' })).not.toBeInTheDocument();
    expect(grouped.getByRole('button', { name: 'Apply' })).toBeVisible();
    expect(grouped.getByRole('button', { name: 'Delete' })).toBeVisible();
  });
});
