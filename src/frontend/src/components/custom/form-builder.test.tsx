import { Input } from '@/components/ui/input';
import { renderCitadel as renderApp } from '@/test/render-citadel';
import { scopedDraftKey } from '@/lib/form-drafts';
import type { ReactElement } from 'react';
const token = `header.${btoa(JSON.stringify({ sub: 'draft-test-user' }))}.signature`;
const renderCitadel = (ui: ReactElement) => renderApp(ui, { auth: { accessToken: token } });
const storageKey = (key: string) => scopedDraftKey(key, token, 'http://localhost')!;
import { screen, waitFor, within } from '@testing-library/react';
import { useState } from 'react';
import { defineField, defineGroupField, FormSchema, FormShell } from './form-builder';

vi.mock('@/lib/monaco', () => ({
  MonacoDiff: () => null,
}));

type TestConfiguration = {
  name: string;
  description: string;
};

const original: TestConfiguration = {
  name: 'api',
  description: 'Public API',
};

const schema: FormSchema<TestConfiguration> = {
  general: {
    title: 'General',
    items: [
      defineField<TestConfiguration, 'name'>({
        key: 'name',
        label: 'Name',
        required: true,
        render: (value, set) => (
          <Input aria-label="Name input" value={value ?? ''} onChange={(event) => set({ name: event.target.value })} />
        ),
      }),
      defineField<TestConfiguration, 'description'>({
        key: 'description',
        label: 'Description',
        render: (value, set) => (
          <Input
            aria-label="Description input"
            value={value ?? ''}
            onChange={(event) => set({ description: event.target.value })}
          />
        ),
      }),
    ],
  },
};

const licensedSchema: FormSchema<TestConfiguration> = {
  automation: {
    title: 'Automation',
    items: [
      defineGroupField<TestConfiguration>({
        id: 'scheduled-backups',
        label: 'Scheduled backups',
        description: 'Run this policy automatically from a cron expression.',
        requiredLicense: 'Team',
        items: [
          defineField<TestConfiguration, 'name'>({
            key: 'name',
            label: 'Name',
            render: (value, set) => (
              <Input
                aria-label="Name input"
                value={value ?? ''}
                onChange={(event) => set({ name: event.target.value })}
              />
            ),
          }),
        ],
      }),
    ],
  },
};

const licensedFieldSchema: FormSchema<TestConfiguration> = {
  automation: {
    title: 'Automation',
    items: [
      defineField<TestConfiguration, 'name'>({
        key: 'name',
        label: 'Redeploy On Build',
        description: 'Automatically redeploy this deployment after the selected build succeeds.',
        requiredLicense: 'Team',
        render: (value, set) => (
          <Input aria-label="Name input" value={value ?? ''} onChange={(event) => set({ name: event.target.value })} />
        ),
      }),
    ],
  },
};

function FormHarness({
  onSave,
  draftKey,
  formSchema = schema,
  confirmSave,
}: {
  onSave: (payload: TestConfiguration) => Promise<void>;
  draftKey?: string;
  formSchema?: FormSchema<TestConfiguration>;
  confirmSave?: (payload: TestConfiguration) => Promise<boolean>;
}) {
  const [update, setUpdate] = useState<Partial<TestConfiguration>>({});

  return (
    <FormShell
      title="Configuration"
      schema={formSchema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={onSave}
      confirmSave={confirmSave}
      draftKey={draftKey}
      draftVersion={1}
    />
  );
}

const enabledSaveButton = () =>
  screen.getAllByRole('button', { name: /save/i }).find((button) => !button.hasAttribute('disabled'));

describe('FormShell', () => {
  beforeEach(() => {
    vi.mocked(window.matchMedia).mockReturnValue({ matches: false } as MediaQueryList);
  });
  it('merges changed fields into the payload and resets the dirty state after saving', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const { user } = renderCitadel(<FormHarness onSave={onSave} />);

    const nameInput = screen.getByRole('textbox', { name: 'Name input' });
    await user.clear(nameInput);
    await user.type(nameInput, 'worker');
    await user.click(enabledSaveButton()!);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith({
        name: 'worker',
        description: 'Public API',
      });
    });
    expect(enabledSaveButton()).toBeUndefined();
    expect(nameInput).toHaveValue('worker');
  });

  it('shows required validation and prevents an invalid save', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const { user } = renderCitadel(<FormHarness onSave={onSave} />);

    await user.clear(screen.getByRole('textbox', { name: 'Name input' }));

    expect(screen.getByText('Required')).toBeVisible();
    expect(enabledSaveButton()).toBeUndefined();
    expect(onSave).not.toHaveBeenCalled();
  });

  it('preserves the current draft when saving fails', async () => {
    const onSave = vi.fn().mockRejectedValue(new Error('save failed'));
    const { user } = renderCitadel(<FormHarness onSave={onSave} draftKey="form-draft" />);

    const descriptionInput = screen.getByRole('textbox', { name: 'Description input' });
    await user.clear(descriptionInput);
    await user.type(descriptionInput, 'Updated description');
    const storedDraft = JSON.parse(localStorage.getItem(storageKey('form-draft'))!);

    await user.click(enabledSaveButton()!);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledOnce();
    });
    expect(JSON.parse(localStorage.getItem(storageKey('form-draft'))!)).toMatchObject({
      version: storedDraft.version,
      update: storedDraft.update,
    });
    expect(descriptionInput).toHaveValue('Updated description');
    expect(enabledSaveButton()).toBeDefined();
  });

  it('preserves the current draft when save confirmation fails', async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const confirmSave = vi.fn().mockRejectedValue(new Error('confirmation failed'));
    const { user } = renderCitadel(
      <FormHarness onSave={onSave} confirmSave={confirmSave} draftKey="form-confirmation-draft" />,
    );

    const descriptionInput = screen.getByRole('textbox', { name: 'Description input' });
    await user.clear(descriptionInput);
    await user.type(descriptionInput, 'Updated description');
    const storedDraft = JSON.parse(localStorage.getItem(storageKey('form-confirmation-draft'))!);

    await user.click(enabledSaveButton()!);

    await waitFor(() => {
      expect(confirmSave).toHaveBeenCalledOnce();
    });
    expect(onSave).not.toHaveBeenCalled();
    expect(JSON.parse(localStorage.getItem(storageKey('form-confirmation-draft'))!)).toMatchObject({
      version: storedDraft.version,
      update: storedDraft.update,
    });
    expect(descriptionInput).toHaveValue('Updated description');
    expect(enabledSaveButton()).toBeDefined();
  });

  it('smoothly scrolls to a field selected from the form navigation', async () => {
    const { user } = renderCitadel(<FormHarness onSave={vi.fn().mockResolvedValue(undefined)} />);
    const fieldset = document.getElementById('name');

    await user.click(screen.getByRole('link', { name: 'Name' }));

    expect(fieldset?.scrollIntoView).toHaveBeenCalledWith({
      block: 'start',
      behavior: 'smooth',
    });
    expect(window.location.hash).toBe('#name');
    expect(screen.getByRole('link', { name: 'Name' })).toHaveAttribute('aria-current', 'location');
    expect(screen.getByRole('link', { name: 'Description' })).not.toHaveAttribute('aria-current');
  });

  it('supports keyboard navigation with reduced motion and preserves router history state', async () => {
    vi.mocked(window.matchMedia).mockReturnValue({ matches: true } as MediaQueryList);
    window.history.replaceState({ key: 'config-page', idx: 3 }, '', '/deployments/edit/test#config');
    const { user } = renderCitadel(<FormHarness onSave={vi.fn().mockResolvedValue(undefined)} />);
    const link = screen.getByRole('link', { name: 'Description' });
    link.focus();
    await user.keyboard('{Enter}');
    expect(document.getElementById('description')?.scrollIntoView).toHaveBeenCalledWith({
      block: 'start',
      behavior: 'auto',
    });
    expect(link).toHaveAttribute('aria-current', 'location');
    expect(window.history.state).toEqual({ key: 'config-page', idx: 3 });
  });

  it('distinguishes edited sections from validation errors and clears status on reset', async () => {
    const { user } = renderCitadel(<FormHarness onSave={vi.fn().mockResolvedValue(undefined)} />);
    const nav = screen.getByRole('navigation', { name: 'Configuration sections' });
    await user.clear(screen.getByRole('textbox', { name: 'Name input' }));
    expect(within(nav).getByRole('link', { name: 'Name, Needs attention' })).toBeVisible();
    await user.type(screen.getByRole('textbox', { name: 'Name input' }), 'worker');
    expect(within(nav).getByRole('link', { name: 'Name, Edited' })).toBeVisible();
    expect(within(nav).queryByText('Needs attention')).not.toBeInTheDocument();
    await user.click(screen.getAllByRole('button', { name: 'Reset' })[0]);
    expect(within(nav).getByRole('link', { name: 'Name' })).toBeVisible();
    expect(within(nav).queryByText('Edited')).not.toBeInTheDocument();
  });

  it('highlights the final section at the scroll boundary and restores the visible section when scrolling up', async () => {
    renderCitadel(
      <div id="main-scroll-container">
        <FormHarness onSave={vi.fn().mockResolvedValue(undefined)} />
      </div>,
    );
    const root = document.getElementById('main-scroll-container')!;
    Object.defineProperties(root, {
      scrollHeight: { configurable: true, value: 1000 },
      clientHeight: { configurable: true, value: 700 },
      scrollTop: { configurable: true, writable: true, value: 0 },
    });
    const first = screen.getByRole('link', { name: 'Name' });
    const last = screen.getByRole('link', { name: 'Description' });
    expect(first).toHaveAttribute('aria-current', 'location');
    root.scrollTop = 300;
    root.dispatchEvent(new Event('scroll'));
    await waitFor(() => expect(last).toHaveAttribute('aria-current', 'location'));
    expect(first).not.toHaveAttribute('aria-current');
    root.scrollTop = 100;
    root.dispatchEvent(new Event('scroll'));
    await waitFor(() => expect(first).toHaveAttribute('aria-current', 'location'));
    expect(last).not.toHaveAttribute('aria-current');
  });

  it('keeps desktop actions outside the scrollable section navigation', async () => {
    const { user } = renderCitadel(<FormHarness onSave={vi.fn().mockResolvedValue(undefined)} />);

    await user.type(screen.getByRole('textbox', { name: 'Description input' }), ' updated');

    const navigation = screen.getByRole('navigation', { name: 'Configuration sections' });
    const actions = document.querySelector('[data-slot="form-sidebar-actions"]');

    expect(navigation).toHaveClass('min-h-0', 'flex-1', 'overflow-y-auto');
    expect(actions).toHaveClass('shrink-0');
    expect(actions?.parentElement).toBe(navigation.parentElement);
  });

  it('shows a license indicator beside a licensed group description', () => {
    renderCitadel(<FormHarness formSchema={licensedSchema} onSave={vi.fn().mockResolvedValue(undefined)} />);

    expect(screen.getByText('Run this policy automatically from a cron expression.')).toBeVisible();
    expect(screen.getByLabelText('Requires a Team license')).toBeVisible();
  });

  it('shows a license indicator beside a licensed field description', () => {
    renderCitadel(<FormHarness formSchema={licensedFieldSchema} onSave={vi.fn().mockResolvedValue(undefined)} />);

    expect(screen.getByText('Automatically redeploy this deployment after the selected build succeeds.')).toBeVisible();
    expect(screen.getByLabelText('Requires a Team license')).toBeVisible();
  });
});

type Credentials = {
  name: string;
  password: string;
  config: { clientSecret: string; endpoint?: string; ports?: number[]; data?: string };
};
const credentialsOriginal: Credentials = { name: '', password: '', config: { clientSecret: '' } };
const credentialsSchema: FormSchema<Credentials> = {
  general: {
    items: [
      defineField<Credentials, 'name'>({
        key: 'name',
        label: 'Name',
        render: (value, set) => (
          <Input aria-label="Resource name" value={value ?? ''} onChange={(e) => set({ name: e.target.value })} />
        ),
      }),
      defineField<Credentials, 'config.endpoint'>({
        key: 'config.endpoint',
        label: 'Endpoint',
        render: (value, set) => (
          <Input
            aria-label="Endpoint"
            value={value ?? ''}
            onChange={(e) =>
              set({ config: { ...credentialsOriginal.config, endpoint: e.target.value, ports: [8080, 9090] } })
            }
          />
        ),
      }),
      defineField<Credentials, 'config.data'>({
        key: 'config.data',
        label: 'Secret data',
        persistDraft: false,
        render: (value, set) => (
          <Input
            aria-label="Secret data"
            value={value ?? ''}
            onChange={(e) => set({ config: { ...credentialsOriginal.config, data: e.target.value } })}
          />
        ),
      }),
      defineField<Credentials, 'password'>({
        key: 'password',
        label: 'Password',
        render: (value, set) => (
          <Input
            aria-label="Password"
            type="password"
            value={value ?? ''}
            onChange={(e) => set({ password: e.target.value })}
          />
        ),
      }),
      defineField<Credentials, 'config.clientSecret'>({
        key: 'config.clientSecret',
        label: 'Client secret',
        render: (value, set) => (
          <Input
            aria-label="Client secret"
            type="password"
            value={value ?? ''}
            onChange={(e) => set({ config: { clientSecret: e.target.value } })}
          />
        ),
      }),
    ],
  },
};
function CredentialsForm({
  onSave = async () => {},
  saved = credentialsOriginal,
}: {
  onSave?: (value: Credentials) => Promise<void>;
  saved?: Credentials;
}) {
  const [update, setUpdate] = useState<Partial<Credentials>>({});
  return (
    <FormShell
      schema={credentialsSchema}
      original={saved}
      update={update}
      setUpdate={setUpdate}
      onSave={onSave}
      draftKey="registry:security-test"
      draftVersion={1}
    />
  );
}

describe('FormShell draft privacy', () => {
  it('retains credentials for submission but restores ordinary settings without persisting credentials', async () => {
    const onSave = vi.fn().mockRejectedValue(new Error('retry later'));
    const { user, unmount } = renderCitadel(<CredentialsForm onSave={onSave} />);
    await user.type(screen.getByLabelText('Resource name'), 'private registry');
    await user.type(screen.getByLabelText('Endpoint', { selector: 'input' }), 'registry.example.com');
    await user.type(screen.getByLabelText('Password', { exact: true, selector: 'input' }), 'password-value');
    await user.type(screen.getByLabelText('Client secret', { exact: true, selector: 'input' }), 'client-secret-value');
    const stored = localStorage.getItem(storageKey('registry:security-test'))!;
    expect(JSON.parse(stored).update).toEqual({
      name: 'private registry',
      config: { endpoint: 'registry.example.com', ports: [8080, 9090] },
    });
    expect(stored).not.toContain('password-value');
    expect(stored).not.toContain('client-secret-value');
    await user.click(enabledSaveButton()!);
    expect(onSave).toHaveBeenCalledWith({
      name: 'private registry',
      password: 'password-value',
      config: { clientSecret: 'client-secret-value', endpoint: 'registry.example.com', ports: [8080, 9090] },
    });
    unmount();
    renderCitadel(<CredentialsForm />);
    expect(screen.getByLabelText('Resource name')).toHaveValue('private registry');
    expect(screen.getByLabelText('Endpoint', { selector: 'input' })).toHaveValue('registry.example.com');
    expect(JSON.parse(localStorage.getItem(storageKey('registry:security-test'))!).update.config.ports).toEqual([
      8080, 9090,
    ]);
    expect(screen.getByLabelText('Password', { exact: true, selector: 'input' })).toHaveValue('');
    expect(screen.getByLabelText('Client secret', { exact: true, selector: 'input' })).toHaveValue('');
  });

  it('does not restore another account’s draft', () => {
    localStorage.setItem(
      storageKey('registry:security-test'),
      JSON.stringify({ version: 1, savedAt: 'now', update: { name: 'alice' } }),
    );
    const otherToken = `header.${btoa(JSON.stringify({ sub: 'other-user' }))}.signature`;
    renderApp(<CredentialsForm />, { auth: { accessToken: otherToken } });
    expect(screen.getByLabelText('Resource name')).toHaveValue('');
    expect(screen.getByLabelText('Password', { exact: true, selector: 'input' })).toHaveValue('');
  });
});

it('discards a draft when its saved fields have changed on the server', async () => {
  const first = renderCitadel(<CredentialsForm />);
  await first.user.type(screen.getByLabelText('Resource name'), 'old local edit');
  first.unmount();
  renderCitadel(<CredentialsForm saved={{ ...credentialsOriginal, name: 'new server name' }} />);
  expect(screen.getByLabelText('Resource name')).toHaveValue('new server name');
  expect(localStorage.getItem(storageKey('registry:security-test'))).toBeNull();
});

it('does not persist original credentials in the draft version or overwrite them when restoring settings', async () => {
  const saved = { name: '', password: 'saved-password', config: { clientSecret: 'saved-client-secret' } };
  const first = renderCitadel(<CredentialsForm saved={saved} />);
  await first.user.type(screen.getByLabelText('Endpoint', { selector: 'input' }), 'registry.example.com');
  const stored = localStorage.getItem(storageKey('registry:security-test'))!;
  expect(stored).not.toContain('saved-password');
  expect(stored).not.toContain('saved-client-secret');
  first.unmount();
  renderCitadel(<CredentialsForm saved={saved} />);
  expect(screen.getByLabelText('Endpoint', { selector: 'input' })).toHaveValue('registry.example.com');
  expect(screen.getByLabelText('Password', { exact: true, selector: 'input' })).toHaveValue('saved-password');
  expect(screen.getByLabelText('Client secret', { exact: true, selector: 'input' })).toHaveValue('saved-client-secret');
});

it('honors explicit secret-field exclusions both when saving and restoring a browser draft', async () => {
  const first = renderCitadel(<CredentialsForm />);
  await first.user.type(screen.getByLabelText('Secret data', { selector: 'input' }), 'opaque-sensitive-content');
  await first.user.type(screen.getByLabelText('Resource name'), 'my registry');
  const key = storageKey('registry:security-test');
  const stored = localStorage.getItem(key)!;
  expect(stored).not.toContain('opaque-sensitive-content');
  first.unmount();
  const draft = JSON.parse(stored);
  draft.update.config = { password: 'injected-password', data: 'injected-data', endpoint: 'registry.example.com' };
  localStorage.setItem(key, JSON.stringify(draft));
  renderCitadel(<CredentialsForm />);
  expect(screen.getByLabelText('Secret data', { selector: 'input' })).toHaveValue('');
  expect(screen.getByLabelText('Endpoint', { selector: 'input' })).toHaveValue('registry.example.com');
  expect(localStorage.getItem(key)).not.toContain('injected-');
});
