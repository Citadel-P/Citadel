import { initializeMonaco } from './setup';

const mocks = vi.hoisted(() => ({
  init: vi.fn(async () => ({})),
  config: vi.fn(),
  yaml: vi.fn(),
  keyValue: vi.fn(),
  stringList: vi.fn(),
}));
vi.mock('@monaco-editor/react', () => ({ loader: { init: mocks.init, config: mocks.config } }));
vi.mock('monaco-editor', () => ({ editor: { createWebWorker: vi.fn() } }));
vi.mock('monaco-yaml', () => ({ configureMonacoYaml: vi.fn() }));
vi.mock('./yaml', () => ({ registerYaml: mocks.yaml }));
vi.mock('./key_value', () => ({ registerKeyValue: mocks.keyValue }));
vi.mock('./string_list', () => ({ registerStringList: mocks.stringList }));

it('shares setup across simultaneous editors and registers each language once', async () => {
  const first = initializeMonaco();
  const second = initializeMonaco();
  expect(second).toBe(first);
  await first;
  await initializeMonaco();
  expect(mocks.config).toHaveBeenCalledOnce();
  expect(mocks.init).toHaveBeenCalledOnce();
  expect(mocks.yaml).toHaveBeenCalledOnce();
  expect(mocks.keyValue).toHaveBeenCalledOnce();
  expect(mocks.stringList).toHaveBeenCalledOnce();
});
