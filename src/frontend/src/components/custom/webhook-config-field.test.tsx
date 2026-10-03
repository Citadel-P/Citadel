import type { WebhookConfig } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { useState } from 'react';
import { WebhookConfigField } from './webhook-config-field';

vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null }));

describe('WebhookConfigField', () => {
  it('uses Web Crypto and configures a shared secret for Generic webhooks', async () => {
    const random = vi.spyOn(globalThis.crypto, 'getRandomValues').mockImplementation((array) => {
      if (array instanceof Uint8Array) array.fill(0xab);
      return array;
    });

    function Harness() {
      const [value, setValue] = useState<WebhookConfig>({ enabled: true });
      return (
        <WebhookConfigField
          resourceType="swarm-service"
          resourceId="service-id"
          execution="update"
          showBranchFilter={false}
          value={value}
          onChange={setValue}
        />
      );
    }

    const { user } = renderCitadel(<Harness />);
    await user.click(screen.getAllByRole('combobox')[0]);
    await user.click(await screen.findByRole('option', { name: 'Generic / CI' }));

    expect(random).toHaveBeenCalledOnce();
    expect(screen.getAllByRole('combobox')[1]).toHaveTextContent('Shared secret (Bearer header)');
    const inputs = screen.getAllByRole('textbox');
    expect(inputs[0]).toHaveValue('ab'.repeat(32));
    expect(inputs[1]).toHaveValue(`${window.location.origin}/listener/generic/swarm-service/service-id/update`);
    expect(screen.getAllByRole('combobox')[0]).toHaveTextContent('Generic / CI');
  });
});
