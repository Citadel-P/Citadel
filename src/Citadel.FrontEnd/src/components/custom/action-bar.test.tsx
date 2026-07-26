import { render, screen } from '@testing-library/react';
import { GenericActionBarButtons } from './action-bar';

describe('GenericActionBarButtons', () => {
  it('uses the responsive grid for grouped actions without standalone actions', () => {
    const StartAction = () => <button type="button">Start</button>;
    const StopAction = () => <button type="button">Stop</button>;

    render(<GenericActionBarButtons resource={{}} actions={[StartAction, StopAction]} />);

    const toolbar = screen.getByRole('button', { name: 'Start' }).closest('[role="group"]')?.parentElement;
    const groupedActions = screen.getByRole('group');

    expect(toolbar).toHaveClass('grid', 'w-full', 'grid-cols-2');
    expect(groupedActions).toHaveClass('grid', 'w-full', 'grid-cols-2');
  });

  it('renders standalone actions outside the grouped actions', () => {
    const ApplyAction = () => <button type="button">Apply</button>;
    const CheckUpdatesAction = () => <button type="button">Check for updates</button>;

    render(<GenericActionBarButtons resource={{}} actions={[ApplyAction]} standaloneActions={[CheckUpdatesAction]} />);

    const apply = screen.getByRole('button', { name: 'Apply' });
    const checkUpdates = screen.getByRole('button', { name: 'Check for updates' });
    const toolbar = checkUpdates.parentElement?.parentElement;
    const groupedActions = apply.closest('[role="group"]');

    expect(toolbar).toHaveClass('grid', 'w-full', 'grid-cols-2');
    expect(groupedActions).toHaveClass('grid', 'w-full', 'grid-cols-2');
    expect(checkUpdates.closest('[role="group"]')).toBeNull();
    expect(checkUpdates.compareDocumentPosition(apply) & Node.DOCUMENT_POSITION_FOLLOWING).not.toBe(0);
  });
});
