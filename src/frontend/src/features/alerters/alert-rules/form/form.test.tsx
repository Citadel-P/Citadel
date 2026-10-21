import { Route, Routes, useParams } from 'react-router';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { AlertRuleStatus, AlertSeverity, AlertType, type AlertRuleConfig } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { AlertRuleFormComponents } from './index';

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => true }),
}));

vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null }));

function ConfigPage() {
  const { id } = useParams();
  const { data } = useRead('getAlertRuleConfig', { id });
  const Config = AlertRuleFormComponents.EditForm!.Tabs[0].Content;
  return data ? <Config resource={data.data} /> : null;
}

it.each([false, true])('respects Write permission in the rule configuration (canWrite=%s)', async (canWrite) => {
  const resource: AlertRuleConfig = {
    id: 'rule-1',
    isSystem: false,
    name: 'License expired',
    type: AlertType.LicenseExpired,
    severity: AlertSeverity.Warning,
    status: AlertRuleStatus.Enabled,
    channelIds: [],
    limitedTo: [],
    quietHours: [],
    capabilities: { canRead: true, canWrite, canExecute: false },
  };
  server.use(
    http.get('http://localhost/api/v1/alertRules/rule-1/_cfg', () => HttpResponse.json(resource)),
    http.get('http://localhost/api/v1/lookup', () => HttpResponse.json([])),
  );
  const { container, user } = renderCitadel(
    <Routes>
      <Route path="/alert-rules/edit/:id" element={<ConfigPage />} />
    </Routes>,
    { route: '/alert-rules/edit/rule-1' },
  );

  const severity = await screen.findByRole('combobox', { name: 'Severity' });
  const status = screen.getByRole('combobox', { name: 'Status' });
  expect(screen.getByRole('combobox', { name: 'Jump to section' })).toBeEnabled();
  if (canWrite) {
    expect(status).toBeEnabled();
    expect(severity).toBeEnabled();
    await user.click(severity);
    await user.click(screen.getByRole('option', { name: 'Critical' }));
    expect(screen.getAllByRole('button', { name: 'Save' })[0]).toBeEnabled();
  } else {
    const controls = container.querySelectorAll('fieldset input, fieldset [role="combobox"], fieldset button');
    expect(controls.length).toBeGreaterThan(2);
    for (const control of controls) expect(control).toBeDisabled();
    for (const save of screen.getAllByRole('button', { name: 'Save' })) expect(save).toBeDisabled();
  }
});
