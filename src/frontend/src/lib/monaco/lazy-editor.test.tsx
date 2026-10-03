import { render, screen, act } from '@testing-library/react';
import { MonacoEditor, MonacoDiff } from './index';

const state = vi.hoisted(() => ({
  imports: 0,
  resolve: () => {},
  ready: new Promise<void>(() => {}),
}));
vi.mock('./editors', () => {
  state.imports++;
  return {
    MonacoEditor: ({ value }: { value: string }) => <div>Editor: {value}</div>,
    MonacoDiff: () => <div>Diff ready</div>,
  };
});
vi.mock('./setup', () => ({ initializeMonaco: () => state.ready }));

it('defers the editor runtime and waits for local setup before mounting editors', async () => {
  expect(state.imports).toBe(0);
  state.ready = new Promise<void>((resolve) => {
    state.resolve = resolve;
  });
  render(
    <>
      <MonacoEditor value="services:" />
      <MonacoDiff original="" modified="" format="text" />
    </>,
  );
  expect(screen.getAllByRole('status')).toHaveLength(2);
  expect(screen.queryByText('Editor: services:')).not.toBeInTheDocument();
  await act(async () => {
    state.resolve();
    await state.ready;
  });
  expect(await screen.findByText('Editor: services:')).toBeVisible();
  expect(await screen.findByText('Diff ready')).toBeVisible();
  expect(state.imports).toBe(1);
});
