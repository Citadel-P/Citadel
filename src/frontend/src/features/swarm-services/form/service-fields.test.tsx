import { screen } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { ServiceReferencesField } from './service-fields';

it.each(['secret', 'config'] as const)('searches %s references and prevents duplicates', async (kind) => {
  const reference = (id: string, name: string) =>
    kind === 'secret'
      ? { secretId: id, secretName: name, targetName: name }
      : { configId: id, configName: name, targetName: name };
  const onChange = vi.fn();
  const { user } = renderCitadel(
    <ServiceReferencesField
      kind={kind}
      resources={[
        { id: 'a', name: 'First' },
        { id: 'b', name: 'Already attached' },
        { id: 'c', name: 'Replacement' },
      ]}
      value={[reference('a', 'First'), reference('b', 'Already attached')]}
      onChange={onChange}
    />,
  );
  await user.click(screen.getByRole('combobox', { name: `${kind} 1` }));
  expect(screen.queryByRole('option', { name: 'None' })).not.toBeInTheDocument();
  const duplicate = screen.getByRole('option', { name: 'Already attached' });
  expect(duplicate).toHaveAttribute('aria-disabled', 'true');
  await user.click(duplicate);
  expect(onChange).not.toHaveBeenCalled();
  await user.type(screen.getByPlaceholderText(`Search ${kind}s...`), 'Replacement');
  await user.click(screen.getByRole('option', { name: 'Replacement' }));
  expect(onChange).toHaveBeenCalledWith([reference('c', 'Replacement'), reference('b', 'Already attached')]);
});
