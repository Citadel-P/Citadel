import { getSwarmComposeDiagnostics } from './swarm-compose-diagnostics';

vi.mock('monaco-editor', () => ({ MarkerSeverity: { Error: 8, Warning: 4 } }));

describe('getSwarmComposeDiagnostics', () => {
  it('accepts a supported Swarm Compose service', () => {
    const diagnostics = getSwarmComposeDiagnostics(`
version: "3.9"
services:
  api:
    image: nginx:latest
    endpoint_mode: vip
    deploy:
      mode: replicated
      replicas: 2
`);

    expect(diagnostics).toEqual([]);
  });

  it('marks Standalone-only keys, reserved labels, and unbound builds', () => {
    const diagnostics = getSwarmComposeDiagnostics(`services:
  api:
    build: .
    container_name: api
    labels:
      com.citadel.owner: forced
`);

    expect(diagnostics).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ lineNumber: 4, message: expect.stringContaining('container_name') }),
        expect.objectContaining({ message: expect.stringContaining('reserved for Citadel ownership') }),
        expect.objectContaining({ message: expect.stringContaining('requires an image') }),
        expect.objectContaining({ message: expect.stringContaining('without a Citadel Build binding') }),
      ]),
    );
  });

  it('accepts an image supplied by a matching Citadel Build binding', () => {
    const diagnostics = getSwarmComposeDiagnostics(
      `services:
  api:
    build: .
`,
      ['API'],
    );

    expect(diagnostics).toEqual([]);
  });

  it('warns about node-local storage, bind mounts, and fixed host ports', () => {
    const diagnostics = getSwarmComposeDiagnostics(`services:
  api:
    image: nginx
    volumes:
      - /srv/api:/data
    ports:
      - target: 80
        published: 8080
        mode: host
volumes:
  data:
    driver: local
`);

    expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual(
      expect.arrayContaining([
        expect.stringContaining('node-local driver'),
        expect.stringContaining('same host path'),
        expect.stringContaining('fixed Host-mode'),
      ]),
    );
  });

  it('marks values outside the backend compatibility matrix', () => {
    const diagnostics = getSwarmComposeDiagnostics(`services:
  api:
    image: nginx
    endpoint_mode: invalid
    deploy:
      update_config:
        failure_action: ignore
`);

    expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual(
      expect.arrayContaining([
        expect.stringContaining("endpoint_mode' has unsupported value 'invalid'"),
        expect.stringContaining("failure_action' has unsupported value 'ignore'"),
      ]),
    );
  });

  it('requires at least one service', () => {
    const diagnostics = getSwarmComposeDiagnostics('services: {}');

    expect(diagnostics).toEqual([expect.objectContaining({ message: 'Compose must define at least one Service.' })]);
  });
});
