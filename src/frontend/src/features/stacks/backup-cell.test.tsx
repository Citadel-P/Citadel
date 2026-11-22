import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { BackupCoverageStatus as Status, type ResourceBackupSummaryView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { StackBackupCell } from './backup-cell';

const coverage: ResourceBackupSummaryView = {
  status: Status.Unprotected,
  platformId: 'platform',
  unavailable: false,
  canConfigure: true,
  hasResourcePolicy: false,
  policies: [],
  warnings: [],
  reasons: [],
  volumes: [{ name: 'wordpress-data', includedInResourcePolicy: true, status: Status.Unprotected, policies: [] }],
};
it('uses the full stack summary, shared hover card and stack configuration link without HTTP', async () => {
  const requests = vi.fn(() => HttpResponse.json({}));
  server.use(http.get('http://localhost/api/v1/stacks/:id', requests));
  const { user, rerender } = renderCitadel(<StackBackupCell stackId="wordpress" />);
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
  rerender(<StackBackupCell stackId="wordpress" coverage={coverage} />);
  await user.hover(screen.getByRole('button', { name: 'Backups: Volumes unprotected' }));
  expect(await screen.findByRole('dialog', { name: 'Backups' })).toBeVisible();
  expect(screen.getByRole('link', { name: 'wordpress-data' })).toBeVisible();
  const link = new URL(
    screen.getByRole('link', { name: 'Configure backup' }).getAttribute('href')!,
    'http://localhost',
  );
  expect(link.searchParams.get('sourceType')).toBe('Stack');
  expect(link.searchParams.get('stackId')).toBe('wordpress');
  expect(link.searchParams.get('returnTo')).toBe('/stacks/edit/wordpress/');
  rerender(<StackBackupCell stackId="wordpress" coverage={{ ...coverage, status: Status.Failed }} />);
  expect(screen.getByRole('button', { name: 'Backups: Backup failed' })).toBeVisible();
  expect(requests).not.toHaveBeenCalled();
});

it('clears coverage when the catalog row changes', () => {
  const { rerender } = renderCitadel(<StackBackupCell stackId="wordpress" coverage={coverage} />);
  rerender(
    <StackBackupCell stackId="wordpress" coverage={{ ...coverage, status: Status.NotApplicable, volumes: [] }} />,
  );
  expect(screen.getByTitle('No eligible storage')).toHaveTextContent('—');
  rerender(<StackBackupCell stackId="other" />);
  expect(screen.getByRole('button', { name: 'Backups: Coverage unavailable' })).toBeVisible();
});
