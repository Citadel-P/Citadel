import { Suspense } from 'react';
import { render, screen } from '@testing-library/react';
import { ResourcePages, ResourceFormPages } from './index';

const imports = vi.hoisted(() => ({ platforms: 0, stacks: 0, form: 0 }));
vi.mock('./platforms', () => {
  imports.platforms++;
  return { PlatformComponents: { label: 'Platforms' } };
});
vi.mock('./stacks', () => {
  imports.stacks++;
  return { StackComponents: { label: 'Stacks' } };
});
vi.mock('./stacks/form', () => {
  imports.form++;
  return { StackFormComponents: { label: 'Stack form' } };
});
vi.mock('@/pages/resource-view', () => ({
  ResourceView: ({ Components }: { Components: { label: string } }) => <div>{Components.label}</div>,
}));
vi.mock('@/pages/resource-form-view', () => ({
  ResourceFormView: ({ Components }: { Components: { label: string } }) => <div>{Components.label}</div>,
}));

it('loads only the selected resource screen and defers its form until opened', async () => {
  expect(imports).toEqual({ platforms: 0, stacks: 0, form: 0 });
  const Platforms = ResourcePages.Platform!;
  const Stacks = ResourcePages.Stack!;
  const Form = ResourceFormPages.Stack!;
  const view = render(
    <Suspense fallback="Loading">
      <Platforms type="Platform" />
    </Suspense>,
  );
  expect(await screen.findByText('Platforms')).toBeVisible();
  expect(imports).toEqual({ platforms: 1, stacks: 0, form: 0 });
  view.rerender(
    <Suspense fallback="Loading">
      <Stacks type="Stack" />
    </Suspense>,
  );
  expect(await screen.findByText('Stacks')).toBeVisible();
  expect(imports).toEqual({ platforms: 1, stacks: 1, form: 0 });
  view.rerender(
    <Suspense fallback="Loading">
      <Form type="Stack" mode="add" />
    </Suspense>,
  );
  expect(await screen.findByText('Stack form')).toBeVisible();
  expect(imports).toEqual({ platforms: 1, stacks: 1, form: 1 });
});
