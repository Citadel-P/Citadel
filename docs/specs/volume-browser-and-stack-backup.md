# Citadel Volume Browser and Stack Backup Source

This specification defines two related capabilities:

1. Browse and download Docker volume contents from Citadel.
2. Add `Stack` as a higher-level backup policy source that resolves to the volumes currently associated with a stack.

Implementation must be delivered in slices.

Unless a slice is explicitly requested, implement **Slice 1 only**.

The stack backup source must not be implemented before the volume-content backend and connector primitives are complete. It must reuse the same volume-discovery and volume-access infrastructure rather than creating a parallel implementation.

---

# 1. Goals

## 1.1 User Goals

Users must be able to:

* Browse the files and directories stored in a Docker volume.
* Navigate the volume using breadcrumb navigation.
* Download a single file as raw file content.
* Download a directory as a tar archive.
* Create a backup policy for an entire stack without manually selecting each underlying Docker volume.
* Preview the volumes currently associated with a stack before creating a backup policy.

## 1.2 Product Goals

Citadel must:

* Treat Docker volumes as inspectable resources rather than opaque Docker names.
* Keep volume-content access read-only for the MVP.
* Work consistently across:

  * Local platforms
  * Regular Agents
  * Edge Agents
* Keep Docker-specific access inside the platform connector and Agent layers.
* Avoid direct access to Docker's host-side volume directories.
* Reuse existing container create, exec, stream, and delete capabilities.
* Keep backup source selection aligned with how users think about workloads:

  * "Back up this stack"
  * rather than "Back up these individual Docker volume names"

---

# 2. Non-Goals

This feature does not implement:

* Editing files inside volumes.
* Uploading files.
* Deleting files.
* Renaming files.
* Arbitrary host filesystem browsing.
* Browsing bind mounts.
* Backing up host bind mounts.
* Backing up Docker networks.
* Backing up images.
* Backing up container writable layers.
* Persisting a historical file index for every volume.
* Full browser-based text or image previews.
* Restoring an entire stack as one atomic transaction.
* Application-consistent multi-volume snapshots.
* Point-in-time consistency across multiple stack volumes.
* Root-volume archive download.
* HTTP range requests.
* File checksums.
* Directory-size calculation.

---

# 3. Current State

Backup policies currently support:

```csharp
public enum BackupSourceType
{
    DockerVolume,
    CitadelSystem
}
```

The existing volume connector supports metadata operations only:

```csharp
public interface IVolumeConnector
{
    Task<Result<IEnumerable<DockerVolumeResult>>> ListVolumesAsync(
        ...);

    Task<Result<DockerVolumeResult>> CreateVolumeAsync(
        ...);

    Task<Result<DockerVolumeResult>> InspectVolumeAsync(
        ...);

    Task<Result> DeleteVolumeAsync(
        ...);
}
```

The container connector already provides or is expected to provide:

* Create container
* Start container
* Execute command
* Stream stdout and stderr
* Inspect exec result
* Delete container

Volume-content operations must not be added to `IVolumeConnector`.

Volume metadata and volume-content access have different:

* Security requirements
* Permission requirements
* Timeout behavior
* Streaming behavior
* Response models
* Cleanup requirements

---

# 4. Architecture

## 4.1 High-Level Flow

Volume browsing uses a temporary helper container on the target Docker platform.

```text
Browser
  -> Citadel Web API
    -> Application Query
      -> IVolumeContentConnector
        -> IContainerConnectorResolver
          -> Local / Agent / Edge Agent connector
            -> Temporary helper container
              -> Read-only volume mount
```

Citadel Core must never:

* Access `/var/lib/docker/volumes`.
* Receive or store the host-side Docker volume mount path.
* Browse the target host filesystem.
* Mount the Docker socket into the helper container.
* Execute arbitrary shell commands supplied by the client.

## 4.2 Connector Abstraction

Add:

```csharp
public interface IVolumeContentConnector
{
    Task<Result<VolumeDirectoryListing>> ListDirectoryAsync(
        ListVolumeDirectoryCommand command,
        CancellationToken cancellationToken);

    Task<Result<VolumeDownloadStream>> OpenDownloadAsync(
        DownloadVolumePathCommand command,
        CancellationToken cancellationToken);
}
```

Do not return only an `IAsyncEnumerable<VolumeDownloadChunk>` from the connector.

The API must know the following before it starts writing response bytes:

* Whether the path exists.
* Whether the path is downloadable.
* Whether it is a file or directory.
* The filename.
* The content type.
* Whether permission and platform validation succeeded.

Use:

```csharp
public sealed class VolumeDownloadStream : IAsyncDisposable
{
    public required string FileName { get; init; }

    public required string ContentType { get; init; }

    public long? ContentLength { get; init; }

    public required IAsyncEnumerable<ReadOnlyMemory<byte>> Chunks
    {
        get;
        init;
    }

    public required Func<ValueTask> CleanupAsync
    {
        get;
        init;
    }

    public ValueTask DisposeAsync()
        => CleanupAsync();
}
```

The connector must complete validation and path inspection before returning the stream object.

The endpoint must dispose the stream after:

* Successful completion
* Client cancellation
* Client disconnection
* Streaming failure
* Unexpected exception

## 4.3 Shared Implementation

Do not duplicate helper-container orchestration for Local, Agent, and Edge Agent.

Prefer one implementation:

```csharp
internal sealed class VolumeContentConnector(
    IContainerConnectorResolver containerConnectorResolver,
    IVolumePathNormalizer pathNormalizer,
    IVolumeHelperImageResolver helperImageResolver,
    ILogger<VolumeContentConnector> logger)
    : IVolumeContentConnector;
```

`IContainerConnectorResolver` selects the appropriate connector based on the platform connector type.

Platform-specific connectors remain responsible for transport:

* Local Docker access
* Regular Agent communication
* Edge Agent communication

The following behavior must exist in one shared volume-content implementation:

* Path normalization
* Permission-independent command construction
* Helper-container configuration
* Helper-container labels
* Cleanup
* Output parsing
* Sorting
* Truncation
* Download metadata
* Error mapping

---

# 5. Helper Runtime

## 5.1 Citadel-Owned Helper

Do not use Alpine shell scripts to generate JSON.

Do not parse `ls`.

Do not generate JSON using `find`, `printf`, `sed`, or shell string interpolation.

Linux filenames may contain:

* Quotes
* Newlines
* Tabs
* Control characters
* Invalid UTF-8 byte sequences

Shell-generated JSON is therefore not sufficiently reliable.

Add a Citadel-owned helper command to the Citadel Agent executable:

```text
citadel-agent volume-helper list
citadel-agent volume-helper inspect
citadel-agent volume-helper stream-file
citadel-agent volume-helper stream-directory
```

The helper must use filesystem APIs and typed serialization.

The helper must not expose a generic arbitrary-command interface.

## 5.2 Helper Image

Use the Citadel helper image corresponding to the running Citadel release and the connector that will execute the helper. Local platforms use the Citadel Core image because it contains `Citadel.VolumeHelper` and `restic`. Agent and Edge Agent platforms use the Citadel Agent image because those connectors create the helper container through the Agent and the Agent image contains `Citadel.Agent.VolumeHelper` and `restic`.

Default helper images:

```text
Local connector:      ghcr.io/citadel-p/citadel:<release-version>
Agent connector:      ghcr.io/citadel-p/citadel.agent:<release-version>
Edge Agent connector: ghcr.io/citadel-p/citadel.agent:<release-version>
```

The helper image must:

* Be pinned to the full `MAJOR.MINOR.PATCH` Citadel release version.
* Be distributed as part of the Citadel release.
* Work in offline and air-gapped installations.
* Contain the native Citadel volume helper executable.
* Not require pulling a public utility image at request time.

Do not use:

```text
alpine:latest
alpine:3.22
busybox
```

as the default helper image.

Image resolution must be internal to the connector or installation configuration.

Do not expose a required environment variable in Slice 1.

An advanced setting may override the helper image for all connectors:

```text
VolumeBrowser__HelperImage=<version-pinned-image>
```

When this override is set, every target Docker daemon must be able to run the configured image.

## 5.3 Helper Commands

### List Directory

Conceptual command:

```text
citadel-agent volume-helper list \
  --root /data \
  --path /config \
  --max-entries 1000 \
  --max-payload-bytes 1048576
```

The helper returns typed JSON.

Suggested response:

```json
{
  "path": "/config",
  "entries": [
    {
      "name": "appsettings.json",
      "path": "/config/appsettings.json",
      "type": "file",
      "size": 4821,
      "modifiedAt": "2026-07-13T14:31:52Z",
      "linkTarget": null
    }
  ],
  "isTruncated": false
}
```

### Inspect Path

Conceptual command:

```text
citadel-agent volume-helper inspect \
  --root /data \
  --path /config/appsettings.json
```

The command must return:

* Exists
* Entry type
* Size when available
* Safe download filename
* Whether the path is allowed for download

### Stream File

Conceptual command:

```text
citadel-agent volume-helper stream-file \
  --root /data \
  --path /config/appsettings.json
```

The helper writes raw file bytes to stdout.

### Stream Directory

Conceptual command:

```text
citadel-agent volume-helper stream-directory \
  --root /data \
  --path /config
```

The helper writes an uncompressed tar stream to stdout.

The tar archive must contain paths relative to the selected directory.

Downloading `/config` should produce entries such as:

```text
config/
config/appsettings.json
config/templates/
```

The helper must not include absolute paths.

---

# 6. Helper Container Security

The temporary helper container must use the following configuration where supported:

```text
Privileged=false
NetworkMode=none
ReadonlyRootfs=true
PidMode=""
IpcMode=""
AutoRemove=false
PidsLimit=64
Memory=128 MiB
MemorySwap=128 MiB
```

Capabilities:

```text
CapDrop=ALL
CapAdd=DAC_READ_SEARCH
```

Security options:

```text
no-new-privileges
```

The helper must:

* Run as root inside the helper container.
* Have no Docker socket mount.
* Have no host filesystem mount.
* Have no host network.
* Have no published ports.
* Have no application-network attachment.
* Mount exactly one Docker volume at `/data`.
* Mount the target volume as read-only.
* Have a read-only root filesystem.
* Avoid access to host PID and IPC namespaces.

`CAP_DAC_READ_SEARCH` is permitted so Citadel can read volume contents owned by arbitrary container UIDs while the volume remains mounted read-only.

No additional capability may be added without an explicit specification change.

---

# 7. Helper Container Lifecycle

Do not combine `AutoRemove=true` with explicit container deletion.

Use:

```text
AutoRemove=false
```

The connector must explicitly remove the helper container.

Create the container with labels:

```text
com.citadel.managed=true
com.citadel.kind=volume-browser
com.citadel.volume=<volume-name>
com.citadel.operation-id=<operation-id>
com.citadel.created-at=<utc-timestamp>
```

Cleanup requirements:

* Remove the helper container in `finally` or `DisposeAsync`.
* Treat Docker `404 Not Found` during cleanup as success.
* Attempt cleanup after cancellation.
* Attempt cleanup after exec failure.
* Attempt cleanup when the client disconnects.
* Log cleanup failure as a warning.
* Never fail an already successful download solely because cleanup returned `404`.

The Agent must clean stale volume-browser helper containers:

* During Agent startup.
* Periodically while the Agent is running.

A helper is stale when:

* It has `com.citadel.kind=volume-browser`.
* Its creation label is older than the configured threshold.
* It is not associated with an active Agent operation.

Suggested defaults:

```text
Helper stale threshold: 15 minutes
Cleanup interval: 5 minutes
```

Internal Citadel helper containers must be ignored by:

* Container synchronization
* Unmanaged-container alerts
* Container activity generation
* User-facing container lists
* Container-count licensing limits

---

# 8. Path Model and Validation

## 8.1 Logical POSIX Paths

All API paths are logical paths inside the selected Docker volume.

The path format is always POSIX-style, regardless of the operating system running Citadel Core.

Valid examples:

```text
/
/config
/config/appsettings.json
/postgres/data/PG_VERSION
```

Invalid examples:

```text
..
/../etc/passwd
/config/../../root
/config//file
/config/./file
C:\data
C:/data
\\server\share
/config\file
```

## 8.2 Core Normalization

Do not use host-dependent methods such as:

```csharp
Path.GetFullPath(...)
```

for API path normalization.

Add:

```csharp
public interface IVolumePathNormalizer
{
    Result<NormalizedVolumePath> Normalize(string? path);
}

public sealed record NormalizedVolumePath(
    string ApiPath,
    IReadOnlyList<string> Segments);
```

Rules:

* `null`, empty, and `/` normalize to `/`.
* Non-root paths must begin with `/`.
* Reject backslashes.
* Reject NUL bytes.
* Reject control characters.
* Reject `.` segments.
* Reject `..` segments.
* Reject repeated separators.
* Reject empty segments except for the root path.
* Reject decoded paths longer than 4096 characters.
* Decode query input exactly once.
* Do not repeatedly URL-decode the path.
* Preserve the logical path using `/` separators.
* Do not convert the path using the Citadel Core operating system.

## 8.3 Symlink Rules

Symlinks must be displayed as entries but must not be followed.

For every requested path, the helper must:

1. Start at `/data`.
2. Resolve one path segment at a time.
3. Use `lstat`-equivalent behavior.
4. Reject the operation if any intermediate segment is a symlink.
5. Reject direct download of a symlink.
6. Return the symlink target only as metadata.

Example:

```text
/data/config/current -> /etc
```

The following must be rejected:

```text
/config/current/appsettings.json
```

even when the lexical path contains no `..` segment.

---

# 9. Permissions

Volume metadata access and volume-content access must use different permissions.

Add specific permissions:

```text
ResourceType.Volume + SpecificPermission.Browse
ResourceType.Volume + SpecificPermission.Download
```

Required permission mapping:

* Volume listing and metadata:

  * Existing volume read/view permission
* Listing files:

  * `Volume.Browse`
* Downloading files or directories:

  * `Volume.Download`

Recommended defaults:

| Role     | View metadata | Browse | Download |
| -------- | ------------: | -----: | -------: |
| Admin    |           Yes |    Yes |      Yes |
| Operator |           Yes |     No |       No |
| Viewer   |           Yes |     No |       No |

Do not allow generic volume metadata read permission to grant volume-content access.

Permission evaluation must happen before helper-container creation.

The selected volume must be evaluated in the context of:

* Platform ID
* Volume name
* Current actor scope
* Resource-level ACL or HCL policy

---

# 10. Output and Resource Limits

## 10.1 Directory Listing Limits

Defaults:

```text
Maximum entries per request: 1000
Maximum decoded path length: 4096 characters
Maximum serialized listing payload: 1 MiB
Default operation timeout: 30 seconds
```

The directory response must include whether the result was truncated.

```csharp
public sealed record VolumeDirectoryListing(
    Guid PlatformId,
    string VolumeName,
    string Path,
    IReadOnlyList<VolumeFileEntry> Entries,
    bool IsTruncated);
```

Pagination is deferred.

The UI must visibly indicate when the result is truncated.

## 10.2 Directory Sorting

Sort entries using:

1. Directory
2. File
3. Symlink
4. Other
5. Name ascending, ordinal-ignore-case
6. Name ascending, ordinal, as a deterministic tie-breaker

## 10.3 Download Limits

Downloads must:

* Stream data.
* Avoid buffering the entire file.
* Avoid buffering the entire tar archive.
* Apply backpressure.
* Support request cancellation.
* Stop the exec operation when cancellation is observed.
* Dispose the helper container after cancellation.
* Keep stdout and stderr separate.
* Verify the exec exit code.

A configurable maximum download size may be added later.

Do not reject large downloads based only on the listing metadata in Slice 1.

---

# 11. Binary-Safe Connector Requirements

Before implementing download, verify that the existing container exec API supports:

* `TTY=false`
* Binary-safe stdout
* Separate stderr
* Streaming without string conversion
* Streaming without line splitting
* Observable final exit code
* Cancellation
* Backpressure
* No full-output buffering

Docker exec streams may use multiplexed stdout and stderr framing.

Any connector that currently converts exec output to:

* UTF-8 strings
* Text lines
* ANSI terminal output
* Log message records

must not be reused directly for file or tar downloads.

If the current connector cannot stream binary output safely, extend the connector and protobuf contracts first.

Suggested abstraction:

```csharp
public sealed record ContainerBinaryExecRequest(
    string ContainerId,
    IReadOnlyList<string> Command,
    IReadOnlyDictionary<string, string>? Environment,
    bool AttachStdout = true,
    bool AttachStderr = true,
    bool Tty = false);

public sealed record ContainerBinaryExecChunk(
    ContainerExecStream Stream,
    ReadOnlyMemory<byte> Data);

public enum ContainerExecStream
{
    Stdout,
    Stderr
}
```

The operation must expose the final exit code separately from stream chunks.

Do not write stderr bytes into the HTTP response body.

---

# 12. API

Use the existing platform-scoped volume route convention.

## 12.1 List Volume Directory

```http
GET /api/v1/platforms/{platformId}/volumes/{name}/files?path=/config
```

Response:

```csharp
public sealed record VolumeDirectoryView(
    Guid PlatformId,
    string VolumeName,
    string Path,
    IReadOnlyList<VolumeFileEntryView> Entries,
    bool IsTruncated);

public sealed record VolumeFileEntryView(
    string Name,
    string Path,
    VolumeFileEntryType Type,
    long? Size,
    DateTimeOffset? ModifiedAt,
    string? LinkTarget = null);

public enum VolumeFileEntryType
{
    Directory,
    File,
    Symlink,
    Other
}
```

Behavior:

* Root path defaults to `/`.
* The selected path must exist.
* The selected path must be a directory.
* A file path returns `400`.
* A symlink path returns `400`.
* A missing path returns `404`.
* An offline platform returns the existing platform-offline error.
* Unsupported connector capability returns a clear capability error.
* Permission failure returns the existing authorization response.

## 12.2 Download Volume Path

```http
GET /api/v1/platforms/{platformId}/volumes/{name}/files/download?path=/config/appsettings.json
```

Behavior:

* Regular file:

  * Return raw file bytes.
* Directory:

  * Return an uncompressed tar archive.
* Symlink:

  * Return `400`.
* Other filesystem type:

  * Return `400`.
* Missing path:

  * Return `404`.
* Root path `/`:

  * Return `400` in the MVP.

Regular file response:

```text
Content-Type: application/octet-stream
Content-Disposition: attachment; filename="<safe-filename>"
```

Directory response:

```text
Content-Type: application/x-tar
Content-Disposition: attachment; filename="<safe-directory-name>.tar"
```

All download responses must include:

```text
Cache-Control: no-store
Pragma: no-cache
X-Content-Type-Options: nosniff
```

Generate `Content-Disposition` using ASP.NET header APIs.

Do not concatenate user-supplied filenames directly into HTTP headers.

## 12.3 Endpoint Registration

Register routes in:

```text
src/Citadel.WebApi/Routes/PublicEndpoints.cs
```

Endpoint methods should live near existing volume endpoints:

```text
src/Citadel.WebApi/Routes/Endpoints/Volumes.cs
```

Keep endpoints thin:

* Bind route and query parameters.
* Send application query through Mediator.
* Map domain results to HTTP responses.
* Configure download headers.
* Stream bytes.
* Dispose download resources.

## 12.4 JSON Source Generation

Add request and response models to:

```text
src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs
```

Regenerate frontend API artifacts after route changes:

```text
src/Citadel.FrontEnd/src/api/generated/api.types.ts
src/Citadel.FrontEnd/src/api/generated/resources.ts
```

---

# 13. Application Layer

Add:

```csharp
public sealed record ListVolumeDirectory(
    Guid PlatformId,
    string VolumeName,
    string Path)
    : IQuery<Result<VolumeDirectoryListing>>;
```

For downloads, prefer a request returning a prepared stream rather than an application-level `IStreamQuery` whose first item contains metadata:

```csharp
public sealed record OpenVolumeDownload(
    Guid PlatformId,
    string VolumeName,
    string Path)
    : IQuery<Result<VolumeDownloadStream>>;
```

The handlers must:

1. Normalize and validate the requested path.
2. Resolve the platform exactly once.
3. Verify that the platform exists.
4. Verify that the platform is online.
5. Verify the requested volume exists on the platform.
6. Evaluate the required permission.
7. Resolve `IVolumeContentConnector`.
8. Call the connector.
9. Return the typed result.

Do not perform database operations inside the download streaming loop.

All of the following must finish before streaming begins:

* Platform lookup
* Volume lookup
* Permission evaluation
* Path normalization
* Connector selection
* Helper-container creation
* Path inspection
* Download metadata resolution

---

# 14. Domain Contracts

Add models under:

```text
src/Citadel.Domain/Contracts/Resources/Volumes
```

Suggested contracts:

```csharp
public sealed record ListVolumeDirectoryCommand(
    string PlatformAddress,
    Guid PlatformId,
    string VolumeName,
    NormalizedVolumePath Path);

public sealed record DownloadVolumePathCommand(
    string PlatformAddress,
    Guid PlatformId,
    string VolumeName,
    NormalizedVolumePath Path);

public sealed record VolumeDirectoryListing(
    Guid PlatformId,
    string VolumeName,
    string Path,
    IReadOnlyList<VolumeFileEntry> Entries,
    bool IsTruncated);

public sealed record VolumeFileEntry(
    string Name,
    string Path,
    VolumeFileEntryType Type,
    long? Size,
    DateTimeOffset? ModifiedAt,
    string? LinkTarget);

public enum VolumeFileEntryType
{
    Directory,
    File,
    Symlink,
    Other
}
```

Do not expose:

* Docker host mount paths
* Helper-container IDs
* Host filesystem paths
* Docker exec IDs
* Internal image references

to the frontend API.

---

# 15. Agent and Edge Agent Support

Local, Agent, and Edge Agent platforms must expose equivalent volume-browser behavior.

The frontend must not silently support only one connector type.

Use existing container operations when they satisfy the binary-streaming requirements:

* Create helper container
* Start helper container
* Execute helper command
* Stream binary stdout
* Read stderr separately
* Observe exit code
* Delete helper container

If protobuf changes are required, update:

```text
src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Protos
```

The same protobuf contract must be consumed by:

* Citadel Core
* Citadel Agent

Do not create separate volume-browser protocols for Agent and Edge Agent.

Both must use the same command and stream models.

The backend may expose a capability flag when an older Agent version does not support binary streaming:

```text
VolumeContentBrowse
VolumeContentDownload
```

The API must return a clear unsupported-agent-version error rather than failing as an internal error.

---

# 16. Frontend UX

## 16.1 Volume List

Add a row action:

```text
Browse
```

The action navigates to:

```text
/platforms/{platformId}/volumes/{encodedVolumeName}/files
```

Volume names must be route-encoded safely.

## 16.2 Volume Details

Add a `Files` tab beside existing volume information.

The tab must contain:

* Breadcrumb navigation
* Current logical path
* Parent-directory action when not at root
* Compact file table
* Loading state
* Empty-directory state
* Truncated-result warning
* Permission-denied state
* Platform-offline state
* Unsupported-Agent-version state
* Download action
* Refresh action

Do not use large marketing cards.

Use the same compact operational styling as existing container, stack, and volume pages.

## 16.3 File Table

Columns:

* Name
* Type
* Size
* Modified
* Actions

Entry behavior:

* Directory:

  * Folder icon
  * Click to navigate
  * Download as tar action
* File:

  * File icon
  * Download action
* Symlink:

  * Link icon
  * Show link target when available
  * No download action
* Other:

  * Generic icon
  * No download action

The UI must not expose:

* Upload
* Edit
* Delete
* Rename
* Extract
* Restore

## 16.4 Live-Data Warning

When the volume is mounted by one or more running containers, show:

> This volume is currently in use. Files may change while browsing, and downloads may not represent a consistent backup.

This warning is informational and does not block browsing.

---

# 17. Activity Logging

Do not create an activity event for:

* Opening the Files tab
* Listing a directory
* Navigating between directories
* Refreshing a directory

Create an activity event after a successful download:

```text
VolumeContentDownloaded
```

Record:

* Actor ID
* Platform ID
* Volume name
* Logical path
* Download type:

  * File
  * DirectoryArchive
* File size when known
* Completion timestamp

Do not record:

* File contents
* Archive contents
* Helper-container ID
* Docker host path
* Downloaded bytes

A failed or cancelled download should not create a successful download activity event.

---

# 18. Stack Backup Source

Implement this section only after the volume-browser backend and connector infrastructure are complete.

## 18.1 Source Type

Add:

```csharp
public enum BackupSourceType
{
    DockerVolume,
    CitadelSystem,
    Stack
}
```

Add:

```csharp
public sealed record StackBackupSource(
    Guid StackId)
    : BackupSourceSpec;
```

The backup policy stores the stack ID.

It must not store a static copy of the volume-name list as its source definition.

The current set of volumes is resolved at:

* Preview time
* Validation time
* Backup execution time

## 18.2 Shared Resolver

Preview and backup execution must use the same resolver.

Add:

```csharp
public interface IStackBackupVolumeResolver
{
    Task<Result<StackBackupVolumeResolution>> ResolveAsync(
        Guid stackId,
        CancellationToken cancellationToken);
}
```

Suggested result:

```csharp
public sealed record StackBackupVolumeResolution(
    Guid StackId,
    string StackName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<ResolvedStackBackupVolume> Volumes,
    IReadOnlyList<string> Warnings);

public sealed record ResolvedStackBackupVolume(
    Guid PlatformId,
    string VolumeName,
    StackVolumeKind Kind,
    bool IsExternal,
    bool IsShared);

public enum StackVolumeKind
{
    DeclaredNamed,
    ExternalNamed,
    AnonymousNamed
}
```

Do not implement separate preview and execution discovery logic.

## 18.3 Persisted Stack Volume Bindings

Persist volume bindings when a stack release applies successfully.

Suggested table or entity:

```csharp
public sealed class StackReleaseVolumeBinding
{
    public Guid Id { get; init; }

    public Guid StackReleaseId { get; init; }

    public Guid PlatformId { get; init; }

    public required string VolumeName { get; init; }

    public string? ComposeVolumeName { get; init; }

    public bool IsExternal { get; init; }

    public bool IsAnonymous { get; init; }
}
```

Bindings should be derived from the successfully created runtime containers and resolved Compose model.

Persisted bindings provide a durable source when:

* The stack is stopped.
* Containers were removed.
* `destroyBeforeDeploy` removed the previous containers.
* Container synchronization has not completed.
* A failed release currently has no running containers.

## 18.4 Discovery Order

The resolver must use:

1. Persisted volume bindings for the current successful stack release.
2. Persisted container mounts associated with the stack.
3. Live Docker container inspection when persisted data is incomplete.
4. Resolved Compose configuration as a fallback.

Do not use Compose YAML parsing as the only source of truth.

Do not list every Docker container on every backup run when persisted ownership data is already available.

## 18.5 Volume Classification

Include:

* Declared Docker named volumes
* External Docker named volumes
* Anonymous Docker volumes mounted by stack-owned containers

Exclude:

* Bind mounts
* Tmpfs mounts
* Container writable layers
* Network resources
* Secret/config mounts that are not Docker volumes

Shared volumes must be included but marked with a warning.

External volumes must be included but marked as external.

Deduplicate by:

```text
(platformId, volumeName)
```

## 18.6 Preview Endpoint

Preferred endpoint:

```http
GET /api/v1/stacks/{stackId}/backup-source-preview
```

Response:

```csharp
public sealed record StackBackupSourcePreviewView(
    Guid StackId,
    string StackName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<StackBackupVolumeView> Volumes,
    IReadOnlyList<string> Warnings);

public sealed record StackBackupVolumeView(
    string Name,
    StackVolumeKind Kind,
    bool IsExternal,
    bool IsShared,
    bool HasBackupCoverage);
```

Use one request per selected stack.

Do not send one preview request per volume.

## 18.7 Backup Run Model

A stack backup creates one logical backup run containing one child item per volume.

Add or extend:

```csharp
public sealed class BackupRunItem
{
    public Guid Id { get; init; }

    public Guid BackupRunId { get; init; }

    public Guid PlatformId { get; init; }

    public required string VolumeName { get; init; }

    public BackupRunItemStatus Status { get; set; }

    public string? SnapshotId { get; set; }

    public string? Error { get; set; }

    public DateTimeOffset StartedAt { get; set; }

    public DateTimeOffset? FinishedAt { get; set; }
}
```

Suggested status:

```csharp
public enum BackupRunItemStatus
{
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled
}
```

Logical run status:

* All items succeeded:

  * `Succeeded`
* Any item failed:

  * `Failed`
* Cancellation occurred:

  * `Cancelled`
* No volumes were resolved:

  * `Failed`

Partial volume success must never result in a successful stack backup run.

## 18.8 Partial Snapshot Behavior

When one volume fails after other volumes succeeded:

* Retain the successful snapshots.
* Associate them with the failed logical run.
* Mark the run as partially completed in its metadata.
* Do not present the run as a successful stack backup.
* Allow administrators to inspect the child volume results.
* Prune retained child snapshots when the failed run is removed by retention or manual cleanup.

Do not silently delete successful snapshots as compensation.

Do not include failed-run snapshots in the normal "latest successful backup" selection.

## 18.9 Retention

Retention applies to the logical backup policy.

When pruning a run, prune all snapshots associated with its `BackupRunItem` records.

This applies to:

* Successful runs
* Failed runs with partial snapshots
* Cancelled runs with partial snapshots

Retention must not prune only the parent database record while leaving Restic snapshots orphaned.

## 18.10 Consistency Guarantee

Stack backup groups multiple volume snapshots into one logical run.

It does not provide:

* Atomic snapshots across volumes
* Application-consistent database backups
* Point-in-time consistency across services
* Automatic database quiescing
* Pre-backup hooks
* Post-backup hooks

Show the following warning in the UI:

> A stack backup groups multiple volume snapshots into one run, but the snapshots are created sequentially and may not represent a single point in time. Application-specific backup hooks are not included in this version.

## 18.11 Restore

Stack-level restore is deferred.

The MVP restore workflow remains volume-level:

* Users can inspect the snapshots created for each volume.
* Users can restore an individual snapshot to a target volume.
* Citadel does not automatically stop stack services.
* Citadel does not restore every stack volume as a group.
* Citadel does not automatically redeploy the stack after restore.

---

# 19. Backup Policy UI

Update the source selector:

```text
Citadel system
Docker volume
Stack
```

When `Stack` is selected:

* Show a stack selector.
* Show the stack platform.
* Show the platform status.
* Request one source preview.
* Show all resolved volumes.
* Show whether each volume is:

  * Declared
  * External
  * Anonymous
  * Shared
* Show warnings.
* Prevent policy creation when no supported volumes are found.
* Prevent policy creation when the stack has no platform.
* Warn when the platform is offline.

The source becomes immutable after the first successful backup run, matching existing source-lock behavior.

Update backup policy and backup run displays so they show:

```text
Source: Stack
Stack: <stack-name>
Volumes: <count>
```

Backup run details must show one row per `BackupRunItem`.

---

# 20. Error Mapping

Map helper and connector failures to stable application errors.

Suggested errors:

```text
VolumeNotFound
VolumePathNotFound
VolumePathInvalid
VolumePathIsNotDirectory
VolumePathIsSymlink
VolumePathTypeUnsupported
VolumeDirectoryListingTooLarge
VolumeContentUnsupportedByAgent
VolumeHelperImageUnavailable
VolumeHelperCreationFailed
VolumeHelperExecFailed
VolumeHelperOutputInvalid
VolumeDownloadFailed
PlatformOffline
PermissionDenied
```

Do not expose raw Docker errors or shell output directly to end users.

Include low-level details in structured logs.

---

# 21. Tests

## 21.1 Path Normalization Unit Tests

Test:

* `null` becomes `/`.
* Empty becomes `/`.
* `/` remains `/`.
* `/config` is accepted.
* `/config/file.json` is accepted.
* `..` is rejected.
* `/../etc` is rejected.
* `/config/../etc` is rejected.
* `/config/./file` is rejected.
* `/config//file` is rejected.
* Backslashes are rejected.
* Windows drive paths are rejected.
* UNC paths are rejected.
* NUL bytes are rejected.
* Control characters are rejected.
* Paths over 4096 characters are rejected.
* Encoded traversal is rejected after one decoding pass.
* Double-encoded input is not repeatedly decoded.

## 21.2 Helper Unit Tests

Test:

* Directory listing includes one level only.
* Files are mapped correctly.
* Directories are mapped correctly.
* Symlinks are reported but not followed.
* Intermediate symlink traversal is rejected.
* Direct symlink download is rejected.
* Other filesystem types are rejected for download.
* Listing truncates at 1000 entries.
* Sorting is deterministic.
* Directory tar paths are relative.
* Raw file streaming does not buffer the complete file.

## 21.3 Application Tests

Test:

* Platform is resolved once.
* Permission is evaluated before helper creation.
* Offline platform fails before helper creation.
* Missing volume fails before helper creation.
* Download performs no database work during streaming.
* Download metadata is available before response streaming.
* Cancellation triggers cleanup.
* Failed exec triggers cleanup.
* Cleanup `404` is treated as success.
* Successful download produces one activity event.
* Cancelled download produces no successful activity event.

## 21.4 Connector Tests

Verify for Local, Agent, and Edge Agent:

* Helper creation
* Read-only volume mount
* Binary stdout preservation
* Separate stderr
* Non-TTY execution
* Observable exit code
* Cancellation
* Container cleanup
* Stale-helper cleanup

## 21.5 Stack Backup Tests

Test:

* Stack source validation.
* Stack-not-found failure.
* Stack-without-platform failure.
* Offline-platform failure.
* No-volume failure.
* Persisted binding resolution.
* Container-mount fallback.
* Live Docker fallback.
* Compose fallback.
* Named-volume deduplication.
* External-volume classification.
* Shared-volume warnings.
* Anonymous-volume inclusion.
* Bind mount exclusion.
* Tmpfs exclusion.
* One child item per volume.
* Logical run succeeds only when every child succeeds.
* Logical run fails when one child fails.
* Partial successful snapshots remain linked to the failed run.
* Retention prunes every child snapshot associated with a pruned run.
* Preview and execution use the same resolver.

## 21.6 Build Verification

Run relevant backend tests.

At minimum, after frontend changes:

```powershell
npm run build:development
```

Regenerate and validate the OpenAPI frontend artifacts after API changes.

---

# 22. Implementation Slices

## Slice 1: Volume Browser Backend for Local Platforms

Implement:

* Domain contracts
* Logical POSIX path normalizer
* Path-normalization tests
* `IVolumeContentConnector`
* Shared `VolumeContentConnector`
* Citadel Agent `volume-helper` command
* Helper-container hardening
* Local container connector integration
* Directory-list endpoint
* Download endpoint
* Raw file streaming
* Directory tar streaming
* Binary exec verification for the Local connector
* Helper cleanup
* Activity event for successful download
* Relevant backend tests
* OpenAPI regeneration
* Frontend API artifact regeneration

Do not implement the frontend browser in Slice 1.

Do not implement stack backup support in Slice 1.

## Slice 2: Agent and Edge Agent Coverage

Implement:

* Binary-safe exec streaming for regular Agent
* Binary-safe exec streaming for Edge Agent
* Shared protobuf changes when required
* Agent capability reporting
* Unsupported-Agent-version error mapping
* Stale-helper cleanup
* Connector behavior tests for:

  * Local
  * Agent
  * Edge Agent

Do not create connector-specific UI behavior.

## Slice 3: Volume Browser Frontend

Implement:

* Browse action on volume rows
* Volume Files route
* Files tab
* Breadcrumb navigation
* Parent-directory navigation
* File table
* File download
* Directory archive download
* Permission-denied state
* Offline-platform state
* Unsupported-Agent-version state
* Truncated-list warning
* Live-volume warning
* Frontend build verification

## Slice 4: Stack Backup Source Backend

Implement:

* `BackupSourceType.Stack`
* `StackBackupSource`
* `StackReleaseVolumeBinding`
* Binding persistence after successful stack apply
* `IStackBackupVolumeResolver`
* Stack backup preview endpoint
* Stack source validation
* Multi-volume backup execution
* `BackupRunItem`
* Partial snapshot handling
* Retention updates
* Backend tests
* OpenAPI regeneration
* Frontend API artifact regeneration

## Slice 5: Stack Backup Source Frontend

Implement:

* `Stack` backup source selector
* Stack selector
* Stack source preview
* Volume classification display
* Warning display
* Policy validation
* Backup policy source display
* Backup run item display
* Partial-failure display
* Frontend build verification

---

# 23. Deferred Work

Future work may add:

* File upload
* File editing
* File deletion
* File rename
* Text preview
* Image preview
* File checksums
* Directory pagination
* Directory-size calculation
* Root archive download
* HTTP range requests
* Configurable maximum download size
* Compression formats such as `.tar.gz`
* Volume export workflows
* Volume import workflows
* Bind-mount browsing with explicit host-path allowlists
* Bind-mount backups
* Deployment backup sources
* Application backup hooks
* Database-aware backup integrations
* Stack-wide grouped restore
* Automated service stop and restart during restore
* Multi-volume point-in-time snapshot integrations
