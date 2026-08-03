import { render, screen } from '@testing-library/react';
import { LayoutContext } from '@/lib/context/layout-context';
import { GenericActionBar, GenericActionBarButtons } from './action-bar';

const layoutContext = {
  theme: { mode: 'light' as const },
  sidebarMinimized: false,
  mobileMenuVisible: false,
  toggleSidebar: vi.fn(),
  setSidebarOpen: vi.fn(),
  toggleMobileMenu: vi.fn(),
  toggleThemeColor: vi.fn(),
  setThemeMode: vi.fn(),
};

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

describe('GenericActionBar', () => {
  it('renders standalone selection actions outside the grouped actions', () => {
    const StartAction = () => <button type="button">Start</button>;
    const AdoptAction = () => <button type="button">Adopt Container</button>;

    render(
      <LayoutContext.Provider value={layoutContext}>
        <GenericActionBar
          selectedItems={[{ id: '1' }]}
          allItems={[{ id: '1' }]}
          resource="Container"
          actions={[StartAction]}
          standaloneActions={[AdoptAction]}
        />
      </LayoutContext.Provider>,
    );

    expect(screen.getByRole('button', { name: 'Adopt Container' }).closest('[role="group"]')).toBeNull();
    expect(screen.getByRole('button', { name: 'Start' }).closest('[role="group"]')).not.toBeNull();
  });

  it.each([
    [false, 'lg:left-[var(--sidebar-width)]'],
    [true, 'lg:left-[var(--sidebar-width-icon)]'],
  ])('uses the available content width when sidebar minimized is %s', (sidebarMinimized, expectedLeftClass) => {
    const DeleteAction = () => <button type="button">Delete</button>;

    render(
      <LayoutContext.Provider value={{ ...layoutContext, sidebarMinimized }}>
        <GenericActionBar
          selectedItems={[{ id: '1' }]}
          allItems={[{ id: '1' }, { id: '2' }]}
          resource="Container"
          actions={[DeleteAction]}
        />
      </LayoutContext.Provider>,
    );

    const actionBar = screen.getByText('1 of 2 container(s) selected.').parentElement;
    const actionGroup = screen.getByRole('group');

    expect(actionBar).toHaveClass('inset-x-0', expectedLeftClass);
    expect(actionBar).toHaveClass('lg:min-h-[var(--layout-footer-height)]');
    expect(actionBar).toHaveClass('border-t', 'border-sidebar-border', 'bg-sidebar');
    expect(actionBar).not.toHaveAttribute('style');
    expect(actionGroup).toHaveClass('grid', 'w-full', 'grid-cols-2', 'gap-2', 'sm:flex', 'sm:w-fit');
    expect(actionGroup.parentElement).toHaveClass('w-full', 'sm:w-auto');
  });
});
