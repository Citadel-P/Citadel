import { StackForm } from './form';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { scopedDraftKey } from '@/lib/form-drafts';

vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useMutate: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useSaveResource: () => ({ save: vi.fn(), isPending: false }),
  useRead: (key: string) => {
    const values: Record<string, unknown> = {
      listPlatforms: { platforms: [] },
      listBuildProjects: { projects: [] },
      lookup: [],
    };
    const data = values[key];
    return { data: data ? { data } : undefined, isFetching: false };
  },
}));
vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => false }),
}));
vi.mock('@/features/tags/components', () => ({ ResourceTagSelector: () => null }));
vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value, onValueChange }: { value: string; onValueChange: (value: string) => void }) => (
    <textarea aria-label="Compose editor" value={value} onChange={(event) => onValueChange(event.target.value)} />
  ),
  MonacoToArrayEditor: () => null,
  MonacoDiff: () => null,
}));
vi.mock('monaco-editor', () => ({ MarkerSeverity: { Error: 8, Warning: 4 } }));

const token = `header.${btoa(JSON.stringify({ sub: 'stack-draft-user' }))}.signature`;
const compose = `services:
  wordpress:
    image: wordpress:php8.3-apache
    restart: unless-stopped
    working_dir: /var/www/html/wordpress
    ports:
      - 127.0.0.1:\${WORDPRESS_PORT:-18404}:80
    environment:
      WORDPRESS_DB_HOST: db:3306
      WORDPRESS_DB_NAME: wordpress
      WORDPRESS_DB_USER: wordpress
      WORDPRESS_DB_PASSWORD: \${WORDPRESS_DB_PASSWORD:?Set a database password in the Stack secret bindings}
      WORDPRESS_PUBLIC_URL: \${WORDPRESS_PUBLIC_URL:?Set the public HTTPS URL ending in /wordpress}
      WORDPRESS_CONFIG_EXTRA: |
        define('DISALLOW_FILE_EDIT', true);
        define('WP_HOME', getenv('WORDPRESS_PUBLIC_URL'));
        define('WP_SITEURL', getenv('WORDPRESS_PUBLIC_URL'));
        define('FORCE_SSL_ADMIN', true);
    volumes:
      - wordpress:/var/www/html/wordpress
    depends_on:
      db:
        condition: service_healthy
  db:
    image: mariadb:11.4
    restart: unless-stopped
    environment:
      MARIADB_DATABASE: wordpress
      MARIADB_USER: wordpress
      MARIADB_PASSWORD: \${WORDPRESS_DB_PASSWORD:?Set a database password in the Stack secret bindings}
      MARIADB_RANDOM_ROOT_PASSWORD: yes
    volumes:
      - database:/var/lib/mysql
    healthcheck:
      test: [CMD, healthcheck.sh, --connect, --innodb_initialized]
      interval: 10s
      timeout: 5s
      retries: 10
      start_period: 30s
volumes:
  wordpress:
    external: true
    name: copied-wordpress
  database:
    external: true
    name: copied-database
`;

it('restores pasted Compose with required secret references and password-generation flags after reload', async () => {
  const first = renderCitadel(<StackForm mode="add" />, { route: '/stacks/add', auth: { accessToken: token } });
  await first.user.click(screen.getByRole('combobox', { name: 'Stack Source' }));
  await first.user.click(screen.getByRole('option', { name: /Web Editor/ }));
  const editor = screen.getByRole('textbox', { name: 'Compose editor' });
  await first.user.click(editor);
  await first.user.keyboard('{Control>}a{/Control}');
  await first.user.paste(compose);
  expect(editor).toHaveValue(compose);
  const key = scopedDraftKey('stack:new', token, 'http://localhost')!;
  expect(JSON.parse(localStorage.getItem(key)!).update.spec.composeFile).toBe(compose);
  first.unmount();
  renderCitadel(<StackForm mode="add" />, { route: '/stacks/add', auth: { accessToken: token } });
  expect(await screen.findByRole('textbox', { name: 'Compose editor' })).toHaveValue(compose);
  expect(screen.getByText(/restored/i)).toBeInTheDocument();
});
