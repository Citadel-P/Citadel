import { maskFormSecrets, sanitizeDraft, scopedDraftKey } from './form-drafts';

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
        script: 'echo ${API_KEY=embedded-secret}',
      }),
    ).toEqual({ environmentVariables: ['PORT=8080', 'DB_PASSWORD=${DATABASE_PASSWORD}'] });
  });

  it.each(['${DB_PASSWORD}', '${DB_PASSWORD:?Set a database password}', '${DB_PASSWORD?Set a database password}'])(
    'retains secret references with required-value messages: %s',
    (reference) => {
      const settings = {
        password: reference,
        environment: [{ key: 'DB_PASSWORD', value: reference }],
        composeFile: `environment:\n  DB_PASSWORD: "${reference}"`,
        script: `DB_PASSWORD='${reference}'`,
      };
      expect(sanitizeDraft(settings)).toEqual(settings);
    },
  );

  it.each(['MARIADB_RANDOM_ROOT_PASSWORD', 'MYSQL_RANDOM_ROOT_PASSWORD'])(
    'retains the password-generation flag %s without treating ordinary passwords as flags',
    (flag) => {
      const settings = {
        environment: { [flag]: 'yes' },
        entries: [{ key: flag, value: '1' }],
        composeFile: `environment:\n  ${flag}: yes`,
      };
      expect(sanitizeDraft(settings)).toEqual(settings);
      expect(sanitizeDraft({ composeFile: `environment:\n  ${flag}: literal-secret` })).toEqual({});
      expect(sanitizeDraft({ composeFile: 'environment:\n  DB_PASSWORD: yes' })).toEqual({});
    },
  );

  it.each(['literal-secret', '${PASSWORD:-literal-secret}', '${PASSWORD}literal-secret', 'prefix${PASSWORD}'])(
    'still excludes literal credential values and reference defaults/suffixes: %s',
    (password) => {
      expect(sanitizeDraft({ password, composeFile: `environment:\n  DB_PASSWORD: "${password}"` })).toEqual({});
    },
  );

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

describe('form diff secret masking', () => {
  it('masks stored and newly entered credentials without mutating the request', () => {
    const value = {
      configuration: {
        userName: 'operator',
        pat: 'new-token',
        password: 'new-password',
        secretAccessKey: 'aws-secret',
      },
      hasClientSecret: true,
      clientSecret: 'oidc-secret',
      privateKey: 'ssh-secret',
      environment: [{ key: 'API_KEY', value: 'env-secret' }],
      opaque: 'custom-secret',
      script: 'TOKEN=embedded-secret',
      retries: 2,
    };
    expect(maskFormSecrets(value, ['opaque'])).toEqual({
      configuration: { userName: 'operator', pat: '******', password: '******', secretAccessKey: '******' },
      hasClientSecret: true,
      clientSecret: '******',
      privateKey: '******',
      environment: [{ key: 'API_KEY', value: '******' }],
      opaque: '******',
      script: '******',
      retries: 2,
    });
    expect(value.configuration.pat).toBe('new-token');
  });
});
