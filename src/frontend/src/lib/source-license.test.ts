import { sourceAndLicenseLinks } from './source-license';

describe('sourceAndLicenseLinks', () => {
  const repositoryUrl = 'https://github.com/Citadel-P/Citadel';
  const revision = 'a'.repeat(40);

  it('links to exact released source and license when informational version embeds the SHA', () => {
    expect(
      sourceAndLicenseLinks({
        repositoryUrl,
        informationalVersion: '1.2.3+height.123.sha.' + revision,
      }),
    ).toEqual({
      sourceUrl: repositoryUrl + '/tree/' + revision,
      licenseUrl: repositoryUrl + '/blob/' + revision + '/LICENSE',
    });
  });

  it('uses the embedded build SHA before authentication', () => {
    expect(sourceAndLicenseLinks({ repositoryUrl, buildRevision: revision })).toEqual({
      sourceUrl: repositoryUrl + '/tree/' + revision,
      licenseUrl: repositoryUrl + '/blob/' + revision + '/LICENSE',
    });
  });

  it('falls back to the repository if no full SHA is present', () => {
    expect(sourceAndLicenseLinks({ repositoryUrl, buildRevision: 'deadbeef' })).toEqual({
      sourceUrl: repositoryUrl,
      licenseUrl: repositoryUrl + '/blob/main/LICENSE',
    });
  });

  it('permits fork builders to configure their own repository', () => {
    expect(
      sourceAndLicenseLinks({ repositoryUrl: 'https://github.com/example/citadel-fork/', buildRevision: revision })
        .sourceUrl,
    ).toBe('https://github.com/example/citadel-fork/tree/' + revision);
  });
});
