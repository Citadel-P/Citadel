import { sanitizeDraft, scopedDraftKey } from './form-drafts';

const token = (sub: string) => `header.${btoa(JSON.stringify({ sub, iss: 'citadel' }))}.signature`;

describe('safe form drafts', () => {
  it('preserves nested settings, lists, scripts, false, zero, null and intentional empty values', () => {
    const settings = {
      platformId: 'platform',
      tagIds: ['tag'],
      enabled: false,
      retries: 0,
      hasClientSecret: true,
      description: null,
      spec: {
        image: { $type: 'ImageTag', registryId: 'registry', imageTag: 'nginx:alpine' },
        ports: [{ hostPort: 8080, containerPort: 80 }],
        volumes: [],
        labels: [{ key: 'team', value: 'ops' }],
        environmentVariables: ['PORT=8080', 'DB_PASSWORD=${DATABASE_PASSWORD}'],
        composeFile: 'services:\n  web:\n    image: nginx:alpine',
      },
      script: 'echo "deploying"',
      empty: {},
      name: '',
    };
    expect(sanitizeDraft(settings)).toEqual(settings);
  });

  it('omits credential values at any depth while retaining secret references and sibling settings', () => {
    const settings = {
      configuration: { $type: 'DockerHub', userName: 'alice', pat: 'pat-value', password: 'password-value' },
      webhook: { enabled: true, secret: 'webhook-value', branchFilter: 'main' },
      config: {
        clientSecret: 'client-value',
        api_key: 'api-value',
        accessToken: 'token-value',
        privateKey: 'key-value',
      },
      environment: { PORT: '8080', DB_PASSWORD: 'db-value' },
      buildArgs: [
        { key: 'MODE', value: 'release' },
        { key: 'API_TOKEN', value: 'build-value' },
      ],
      secrets: [{ secretId: 'secret-id', target: '/run/secrets/database' }],
      passwordSecretId: 'password-secret-id',
    };
    expect(sanitizeDraft(settings)).toEqual({
      configuration: { $type: 'DockerHub', userName: 'alice' },
      webhook: { enabled: true, branchFilter: 'main' },
      environment: { PORT: '8080' },
      buildArgs: [{ key: 'MODE', value: 'release' }, { key: 'API_TOKEN' }],
      secrets: [{ secretId: 'secret-id', target: '/run/secrets/database' }],
      passwordSecretId: 'password-secret-id',
    });
    expect(settings.configuration.password).toBe('password-value');
  });

  it('excludes explicitly marked opaque data without excluding its siblings', () => {
    expect(sanitizeDraft({ name: 'secret', spec: { data: 'opaque', labels: ['ops'] } }, ['spec.data'])).toEqual({
      name: 'secret',
      spec: { labels: ['ops'] },
    });
  });

  it('filters common inline credentials but keeps normal environment values and secret references', () => {
    expect(
      sanitizeDraft({
        environmentVariables: ['PORT=8080', 'API_KEY=raw-value', 'DB_PASSWORD=${DATABASE_PASSWORD}'],
        endpoint: 'https://alice:password@example.com',
        callback: 'https://example.com?api_key=raw-value',
        composeFile: 'environment:\n  DB_PASSWORD: "raw-value"',
        privateKeyFile: '-----BEGIN OPENSSH PRIVATE KEY-----\nkey',
      }),
    ).toEqual({ environmentVariables: ['PORT=8080', 'DB_PASSWORD=${DATABASE_PASSWORD}'] });
  });

  it('does not restore prototype keys or create empty drafts for credential-only changes', () => {
    const draft = JSON.parse(
      '{"__proto__":{"polluted":true},"config":{"constructor":{"prototype":{"polluted":true}},"password":"secret"}}',
    );
    expect(sanitizeDraft(draft)).toEqual({});
    expect({}).not.toHaveProperty('polluted');
  });

  it('isolates accounts, servers and resources without storing the token', () => {
    const key = scopedDraftKey('platform:one', token('alice'), 'server-a');
    expect(key).not.toEqual(scopedDraftKey('platform:one', token('bob'), 'server-a'));
    expect(key).not.toEqual(scopedDraftKey('platform:one', token('alice'), 'server-b'));
    expect(key).not.toEqual(scopedDraftKey('platform:two', token('alice'), 'server-a'));
    expect(key).not.toContain(token('alice'));
    expect(scopedDraftKey('platform:one', 'invalid', 'server-a')).toBeUndefined();
    expect(scopedDraftKey('platform:one', undefined, 'server-a')).toBeUndefined();
  });
});
