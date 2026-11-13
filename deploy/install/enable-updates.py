#!/usr/bin/env python3
"""Enable the existing Citadel updater on a local Linux Compose installation.

Run after first-run setup. This does not install a release or update PostgreSQL.
The packaged Rust registration/check commands remain the authority for eligibility.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import tempfile
import uuid

SETTING = "CITADEL_UPDATE_STATE_DIRECTORY"


class SetupError(Exception):
    pass


def private_path(path, directory=False, private=False):
    """Refuse symlinks and foreign/writable ancestors before privileged I/O."""
    if not path.is_absolute() or '..' in path.parts or ',' in str(path) or any(c.isspace() for c in str(path)):
        raise SetupError("Use absolute paths without whitespace, commas or '..'.")
    for part in [*reversed(path.parents), path]:
        info = part.lstat()
        leaf = part == path
        sticky = not leaf and info.st_uid == 0 and bool(info.st_mode & stat.S_ISVTX)
        if (stat.S_ISLNK(info.st_mode) or info.st_uid not in (0, os.geteuid())
                or (info.st_mode & 0o022 and not sticky)):
            raise SetupError("Installer/state paths must have trusted owners, no symlinks and no group/other write access.")
        if not leaf or directory:
            if not stat.S_ISDIR(info.st_mode):
                raise SetupError("Expected a directory.")
        elif not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
            raise SetupError("Expected a regular file without hard links.")
        if leaf and private and info.st_mode & 0o077:
            raise SetupError("Use mode 0700 for update state and 0600 for .env; existing permissions were not changed.")
    return path


def read_environment(path):
    private_path(path, private=True)
    if path.stat().st_size > 1024 * 1024:
        raise SetupError("The installer .env is too large.")
    raw = path.read_bytes()
    values = {}
    for line in raw.decode('utf-8').splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        name, separator, value = line.partition('=')
        if not separator or not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', name) or name in values:
            raise SetupError("Use unique NAME=value assignments in .env.")
        values[name] = value
    return raw, values


def run(args, *, cwd, environment, step, timeout=180):
    try:
        result = subprocess.run(args, cwd=cwd, env=environment, capture_output=True,
                                text=True, check=True, timeout=timeout)
        return result.stdout.strip()
    except (subprocess.SubprocessError, OSError) as error:
        # Compose errors/output can contain interpolated credentials.
        raise SetupError(f"{step} failed. No update was approved. Preserve .env and the update-state directory; see the update setup recovery guide.") from error


def enable(args):
    directory = private_path(Path(args.directory), directory=True)
    files = [private_path(Path(value)) for value in args.compose_file] if args.compose_file else [private_path(directory / 'docker-compose.yml')]
    if not 1 <= len(files) <= 4 or len(set(files)) != len(files) or any(p.parent != directory for p in files):
        raise SetupError("Supply the original one to four Compose files, in order, directly inside the installation directory.")
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]*', args.project):
        raise SetupError("Supply the existing Compose project name; do not choose a new name.")
    state = Path(args.state_directory) if args.state_directory else directory.with_name(directory.name + '-update-state')
    private_path(state.parent, directory=True)
    if state == directory or state in directory.parents or directory in state.parents:
        raise SetupError("Use a separate update-state directory outside the installation directory.")
    env_path = directory / '.env'
    original, settings = read_environment(env_path)
    configured = settings.get(SETTING)
    if configured is not None and configured != str(state):
        raise SetupError("The existing update-state setting differs; it was not replaced.")
    # Do not let shell variables silently override the protected Compose inputs.
    environment = {key: value for key, value in os.environ.items() if key in (
        'PATH', 'HOME', 'LANG', 'LC_ALL', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'XDG_RUNTIME_DIR')}
    def command(arguments, step, timeout=180):
        return run(arguments, cwd=directory, environment=environment, step=step, timeout=timeout)
    context = environment.get('DOCKER_CONTEXT')
    if context or not environment.get('DOCKER_HOST'):
        endpoint = command(['docker', 'context', 'inspect', *([context] if context else []), '--format', '{{.Endpoints.docker.Host}}'], 'Docker context inspection')
    else:
        endpoint = environment['DOCKER_HOST']
    if not endpoint.startswith('unix:///'):
        raise SetupError("Update setup requires a local Linux Docker socket, not a remote daemon.")
    socket = endpoint.removeprefix('unix://')
    if not stat.S_ISSOCK(Path(socket).stat().st_mode):
        raise SetupError("The local Docker endpoint is not a socket.")
    docker = ['docker', '--host', endpoint]
    # Explicit --host pins all subsequent commands to the inspected daemon.
    environment.pop('DOCKER_CONTEXT', None)
    environment.pop('DOCKER_HOST', None)
    info = json.loads(command([*docker, 'info', '--format', '{{json .}}'], 'Docker inspection'))
    rootless = 'name=rootless' in info.get('SecurityOptions', [])
    if info.get('OSType') != 'linux' or 'desktop' in info.get('OperatingSystem', '').lower():
        raise SetupError("Use a native Linux Docker host; Docker Desktop installations require manual updates.")
    if not rootless and os.geteuid() != 0:
        raise SetupError("Rootful Docker requires running this command as root with root-owned installation inputs. See the update setup guide.")
    if rootless and socket != f'/run/user/{os.geteuid()}/docker.sock':
        raise SetupError("Run setup as the rootless Docker daemon's host user.")
    if rootless:
        for path in [directory, env_path, *files]:
            if path.stat().st_uid != os.geteuid():
                raise SetupError("Rootless installation inputs must belong to the daemon's host user.")
    docker_root = Path(info['DockerRootDir'])
    if state == docker_root or docker_root in state.parents:
        raise SetupError("Keep update state outside Docker's data directory.")
    compose = [*docker, 'compose', '--project-name', args.project, '--project-directory', str(directory), '--env-file', str(env_path)]
    for path in files:
        compose += ['-f', str(path)]
    def inspect_service(service):
        ids = command([*compose, 'ps', '-q', service], 'Compose service inspection').splitlines()
        if len(ids) != 1:
            raise SetupError("Start the existing Compose installation and finish first-run setup before enabling updates.")
        return json.loads(command([*docker, 'inspect', ids[0]], 'Container inspection'))[0]
    core, database = inspect_service('server'), inspect_service('pg_db')
    image = core['Image']  # Immutable installed image ID; never pull a newer release.
    if not re.fullmatch(r'sha256:[a-f0-9]{64}', image):
        raise SetupError("Cannot establish the installed Core image identity.")
    if not state.exists():
        state.mkdir(mode=0o700)
    private_path(state, directory=True, private=True)
    helper = [*docker, 'run', '--rm', '--pull', 'never', '--network', 'none', '--user', '0:0',
              '--read-only', '--tmpfs', '/tmp:rw,nosuid,nodev', '--entrypoint', '/app/citadel-server',
              '--mount', f'type=bind,src={directory},dst={directory},readonly',
              '--mount', f'type=bind,src={state},dst={state}',
              '--mount', f'type=bind,src={socket},dst=/var/run/docker.sock', image]
    def helper_command(arguments, step):
        identity = str(uuid.uuid4())
        name = 'citadel-update-setup-' + identity
        # Insert run options after `docker --host ENDPOINT run`.
        invocation = [*helper[:4], '--name', name, '--label', 'com.citadel.update-setup=' + identity, *helper[4:]]
        try:
            return command([*invocation, *arguments], step)
        except (SetupError, KeyboardInterrupt):
            try:
                owner = command([*docker, 'inspect', '--format', '{{index .Config.Labels "com.citadel.update-setup"}}', name], 'Setup helper inspection', timeout=15)
                if owner == identity:
                    command([*docker, 'rm', '--force', name], 'Setup helper cleanup', timeout=30)
            except SetupError:
                print(f'If retained, inspect setup helper {name}. Do not delete update-state files.', file=sys.stderr)
            raise
    register = ['register-update-installation', '--directory', str(directory), '--project', args.project,
                '--state-directory', str(state)]
    for path in files:
        register += ['--compose-file', str(path)]
    check = ['check-update-installation', '--state-directory', str(state)]
    # Rust refuses drift, unsafe mounts, unfinished setup, and active/orphaned
    # recovery evidence. Never refresh an unknown previous configuration.
    helper_command(register, 'Installation registration')
    helper_command(check, 'Installation and recovery preflight')
    runtime_env = core['Config'].get('Env', [])
    if configured:
        if f'{SETTING}={state}' not in runtime_env:
            raise SetupError("The setting exists but Core is not using it. Complete the documented interrupted-setup procedure; no automatic retry was attempted.")
        return 'In-app updates are already configured and verified.'
    if any(value.startswith(SETTING + '=') for value in runtime_env):
        raise SetupError("Core's update-state setting is not owned by .env. Keep the existing configuration and review it manually.")
    selected_image = command([*docker, 'image', 'inspect', core['Config']['Image'], '--format', '{{.Id}}'], 'Installed image verification')
    if selected_image != image:
        raise SetupError("The local Core image tag no longer points to the running image. Restore the installed image reference before setup; no restart was attempted.")
    # Preserve operator bytes and use atomic publication; never source .env.
    if read_environment(env_path)[0] != original:
        raise SetupError(".env changed during setup; registration was preserved and Core was not restarted.")
    temporary = None
    try:
        fd, temporary = tempfile.mkstemp(prefix='.env.update-setup-', dir=directory)
        with os.fdopen(fd, 'wb') as output:
            output.write(original + (b'' if original.endswith(b'\n') else b'\n') + f'{SETTING}={state}\n'.encode())
            output.flush()
            os.fsync(output.fileno())
        if read_environment(env_path)[0] != original:
            raise SetupError(".env changed during setup; it was not replaced.")
        os.replace(temporary, env_path)
        temporary = None
        parent = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    finally:
        if temporary is not None:
            Path(temporary).unlink(missing_ok=True)
    print('Registration verified. Restarting only Core to enable in-app updates…', flush=True)
    command([*compose, 'up', '-d', '--no-deps', '--no-build', '--pull', 'never', '--wait', '--wait-timeout', '180', 'server'], 'Core restart', timeout=240)
    current_core, current_database = inspect_service('server'), inspect_service('pg_db')
    if (current_core['Image'] != image or current_database['Id'] != database['Id']
            or current_database['State']['StartedAt'] != database['State']['StartedAt']
            or current_database['RestartCount'] != database['RestartCount']):
        raise SetupError("Container identity changed unexpectedly. Preserve registration and configuration for inspection; no refresh was attempted.")
    # Explicitly accept only the Core/configuration change just made above.
    # Rust preserves instance, daemon, database, volumes, network and registration ID.
    helper_command([*register, '--refresh'], 'Registration refresh')
    helper_command(check, 'Final installation verification')
    return 'In-app Core and PostgreSQL updates are configured. No release or database upgrade was installed.'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', required=True, help='Original absolute Compose installation directory')
    parser.add_argument('--project', required=True, help='Existing Compose project name')
    parser.add_argument('--compose-file', action='append', default=[], help='Original absolute Compose path; repeat in original order')
    parser.add_argument('--state-directory', help='Private host directory; defaults to <installation-directory>-update-state')
    args = parser.parse_args()
    def interrupted(*_):
        raise KeyboardInterrupt()
    signal.signal(signal.SIGTERM, interrupted)
    try:
        if sys.platform != 'linux':
            raise SetupError('Run update setup on the Linux Docker host.')
        directory = private_path(Path(args.directory), directory=True)
        lock_path = directory / '.update-setup.lock'
        fd = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'r+b') as lock:
            private_path(lock_path, private=True)
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            print(enable(args))
    except KeyboardInterrupt:
        print('Setup interrupted. Preserve .env and update-state files; verify Core health before retrying.', file=sys.stderr)
        return 130
    except (SetupError, OSError, ValueError, KeyError) as error:
        # Never echo malformed environment/configuration values or Docker output.
        message = str(error) if isinstance(error, SetupError) else 'Setup could not finish. Check paths, permissions and the Docker connection. Preserve existing registration and recovery state.'
        print(message, file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
