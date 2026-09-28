import { pickDraftFields, scopedDraftKey } from './form-drafts';

const token = (sub: string) => `header.${btoa(JSON.stringify({ sub, iss: 'citadel' }))}.signature`;

describe('safe form drafts', () => {
  it('keeps only explicitly allowed scalar fields, including nested fields', () => {
    expect(
      pickDraftFields(
        {
          name: 'app',
          config: { endpoint: 'docker.sock', password: 'secret' },
          environment: { TOKEN: 'secret' },
          script: 'secret',
        },
        ['name', 'config.endpoint', 'environment'],
      ),
    ).toEqual({ name: 'app', config: { endpoint: 'docker.sock' } });
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
