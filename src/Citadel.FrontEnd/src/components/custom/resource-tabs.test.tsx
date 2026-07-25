import { screen } from '@testing-library/react';
import { ResourceTabElement } from '@/pages/types';
import { renderCitadel } from '@/test/render-citadel';
import { ResourceTabs } from './resource-tabs';

type TestResource = {
  name: string;
  description: string | null;
  status: string;
  logsEnabled: boolean;
};

const resource: TestResource = {
  name: 'api',
  description: null,
  status: 'Running',
  logsEnabled: true,
};

const tabs: ResourceTabElement<TestResource>[] = [
  {
    label: 'Overview',
    Content: () => <div>overview content</div>,
  },
  {
    label: 'Logs',
    disabled: (item) => !item.logsEnabled,
    Content: () => <div>log output</div>,
  },
  {
    label: 'Inspect',
    Content: () => <div>inspect content</div>,
  },
];

describe('ResourceTabs', () => {
  it('switches tabs, updates the URL hash, and restores the selected tab', async () => {
    const firstRender = renderCitadel(
      <ResourceTabs localKey="resource-tab" resource={resource} tabs={tabs} />,
      { route: '/containers/1' },
    );

    expect(screen.getByText('overview content')).toBeVisible();

    await firstRender.user.click(screen.getByRole('tab', { name: 'Logs' }));

    expect(screen.getByText('log output')).toBeVisible();
    expect(localStorage.getItem('resource-tab')).toBe(JSON.stringify('Logs'));
    firstRender.unmount();

    renderCitadel(<ResourceTabs localKey="resource-tab" resource={resource} tabs={tabs} />, {
      route: '/containers/1',
    });

    expect(screen.getByRole('tab', { name: 'Logs' })).toHaveAttribute('data-state', 'active');
    expect(screen.getByText('log output')).toBeVisible();
  });

  it('renders log content again after switching to another tab and back', async () => {
    const { user } = renderCitadel(
      <ResourceTabs localKey="resource-tab" resource={resource} tabs={tabs} />,
      { route: '/containers/1#logs' },
    );

    expect(screen.getByText('log output')).toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Inspect' }));
    expect(screen.getByText('inspect content')).toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Logs' }));
    expect(screen.getByText('log output')).toBeVisible();
  });

  it('falls back to the first enabled tab when the stored tab becomes disabled', async () => {
    localStorage.setItem('resource-tab', JSON.stringify('Logs'));

    renderCitadel(
      <ResourceTabs
        localKey="resource-tab"
        resource={{ ...resource, logsEnabled: false }}
        tabs={tabs}
      />,
      { route: '/containers/1' },
    );

    expect(screen.getByRole('tab', { name: 'Logs' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Overview' })).toHaveAttribute('data-state', 'active');
    expect(screen.getByText('overview content')).toBeVisible();
  });

  it('uses a valid URL hash in preference to the stored tab', () => {
    localStorage.setItem('resource-tab', JSON.stringify('Overview'));

    renderCitadel(<ResourceTabs localKey="resource-tab" resource={resource} tabs={tabs} />, {
      route: '/containers/1#inspect',
    });

    expect(screen.getByRole('tab', { name: 'Inspect' })).toHaveAttribute('data-state', 'active');
    expect(screen.getByText('inspect content')).toBeVisible();
  });
});
