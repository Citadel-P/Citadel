import { Input } from '@/components/ui/input';
import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor } from '@testing-library/react';
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
          <Input
            aria-label="Name input"
            value={value ?? ''}
            onChange={(event) => set({ name: event.target.value })}
          />
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
          <Input
            aria-label="Name input"
            value={value ?? ''}
            onChange={(event) => set({ name: event.target.value })}
          />
        ),
      }),
    ],
  },
};

function FormHarness({
  onSave,
  draftKey,
  formSchema = schema,
}: {
  onSave: (payload: TestConfiguration) => Promise<void>;
  draftKey?: string;
  formSchema?: FormSchema<TestConfiguration>;
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
      draftKey={draftKey}
      draftVersion={1}
    />
  );
}

const enabledSaveButton = () =>
  screen.getAllByRole('button', { name: /save/i }).find((button) => !button.hasAttribute('disabled'));

describe('FormShell', () => {
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
    const storedDraft = JSON.parse(localStorage.getItem('form-draft')!);

    await user.click(enabledSaveButton()!);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledOnce();
    });
    expect(JSON.parse(localStorage.getItem('form-draft')!)).toMatchObject({
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
  });

  it('shows a license indicator beside a licensed group description', () => {
    renderCitadel(<FormHarness formSchema={licensedSchema} onSave={vi.fn().mockResolvedValue(undefined)} />);

    expect(screen.getByText('Run this policy automatically from a cron expression.')).toBeVisible();
    expect(screen.getByLabelText('Requires a Team license')).toBeVisible();
  });

  it('shows a license indicator beside a licensed field description', () => {
    renderCitadel(<FormHarness formSchema={licensedFieldSchema} onSave={vi.fn().mockResolvedValue(undefined)} />);

    expect(
      screen.getByText('Automatically redeploy this deployment after the selected build succeeds.'),
    ).toBeVisible();
    expect(screen.getByLabelText('Requires a Team license')).toBeVisible();
  });
});
