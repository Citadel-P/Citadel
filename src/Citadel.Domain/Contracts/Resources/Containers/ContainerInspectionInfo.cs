using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public record ContainerInspectionInfo(
    string Id,
    string Created,
    string? Path,
    IReadOnlyList<string> Args,
    ContainerRuntimeState State,
    string? Image,
    string? ResolvConfPath,
    string? HostnamePath,
    string? HostsPath,
    string? LogPath,
    string? Name,
    int? RestartCount,
    string? Driver,
    string? Platform,
    string? MountLabel,
    string? ProcessLabel,
    string? AppArmorProfile,
    IReadOnlyList<string> ExecIDs,
    HostConfiguration? HostConfig,
    GraphDriverDataInfo? GraphDriver,
    long? SizeRw,
    long? SizeRootFs,
    IReadOnlyList<MountPointInfo> Mounts,
    ContainerConfiguration? Config,
    NetworkSettingsInfo? NetworkSettings
);

public record ContainerHealthStatus(string? Status, int? FailingStreak);

public record LogConfiguration(string? Type, IReadOnlyDictionary<string, string> Config);

public record ContainerRuntimeState(
    ContainerStateStatus Status,
    bool? Running,
    bool? Paused,
    bool? Restarting,
    bool? OOMKilled,
    bool? Dead,
    int? Pid,
    int? ExitCode,
    string? Error,
    string? StartedAt,
    string? FinishedAt,
    ContainerHealthStatus? Health
);

public record RestartPolicy(string? Name, int? MaximumRetryCount);

public record BindOptions(
    string? Propagation,
    bool? NonRecursive,
    bool? CreateMountpoint,
    bool? ReadOnlyNonRecursive,
    bool? ReadOnlyForceRecursive
);

public record DriverConfiguration(string? Name, IReadOnlyDictionary<string, string> Options);

public record VolumeOptions(
    bool? NoCopy,
    IReadOnlyDictionary<string, string> Labels,
    DriverConfiguration? DriverConfig,
    string? Subpath);

public record HostMount(
    string? Target,
    string? Source,
    string? Type,
    bool? ReadOnly,
    string? Consistency,
    BindOptions? BindOptions,
    VolumeOptions? VolumeOptions
);

public record Ulimit(string? Name, long? Soft, long? Hard);

public record HostConfiguration(
    IReadOnlyList<string> Binds,
    string? ContainerIDFile,
    LogConfiguration? LogConfig,
    string? NetworkMode,
    IDictionary<string, IReadOnlyList<HostPortBinding>> PortBindings,
    RestartPolicy? RestartPolicy,
    bool? AutoRemove,
    string? VolumeDriver,
    IReadOnlyList<string> VolumesFrom,
    IReadOnlyList<HostMount> Mounts,
    IReadOnlyList<int> ConsoleSize,
    IReadOnlyDictionary<string, string> Annotations,
    IReadOnlyList<string> CapAdd,
    IReadOnlyList<string> CapDrop,
    string? CgroupnsMode,
    IReadOnlyList<string> Dns,
    IReadOnlyList<string> DnsOptions,
    IReadOnlyList<string> DnsSearch,
    IReadOnlyList<string> ExtraHosts,
    IReadOnlyList<string> GroupAdd,
    string? IpcMode,
    string? Cgroup,
    IReadOnlyList<string> Links,
    int? OomScoreAdj,
    string? PidMode,
    bool? Privileged,
    bool? PublishAllPorts,
    bool? ReadonlyRootfs,
    IReadOnlyList<string> SecurityOpt,
    IReadOnlyDictionary<string, string> StorageOpt,
    IReadOnlyDictionary<string, string> Tmpfs,
    string? UtsMode,
    string? UsernsMode,
    long ShmSize,
    IReadOnlyDictionary<string, string> Sysctls,
    string? Runtime,
    string? Isolation,
    IReadOnlyList<string> MaskedPaths,
    IReadOnlyList<string> ReadonlyPaths,
    long? MemorySwap,
    long? MemorySwappiness,
    long? NanoCpus,
    long? PidsLimit,
    long? Memory,
    long? MemoryReservation,
    long? IoMaximumBandwidth,
    long? CpuPeriod,
    long? CpuPercent,
    long? CpuCount,
    IReadOnlyList<Ulimit> Ulimits,
    long? KernelMemoryTCP);

public record GraphDriverDataInfo(string? Name, IReadOnlyDictionary<string, string> Data);

public record MountPointInfo(
    string? Type,
    string? Name,
    string? Source,
    string? Destination,
    string? Driver,
    string? Mode,
    bool? RW,
    string? Propagation
);

public record ContainerConfiguration(
    string? Hostname,
    string? Domainname,
    string? User,
    bool? AttachStdin,
    bool? AttachStdout,
    bool? AttachStderr,
    IReadOnlyList<string>? ExposedPorts,
    bool? Tty,
    bool? OpenStdin,
    bool? StdinOnce,
    IReadOnlyList<string> Env,
    IReadOnlyList<string> Cmd,
    string? Image,
    IReadOnlyList<string>? Volumes,
    string? WorkingDir,
    IReadOnlyList<string> Entrypoint,
    bool? NetworkDisabled,
    string? MacAddress,
    IReadOnlyList<string> OnBuild,
    IReadOnlyDictionary<string, string> Labels);

public record IpAddressInfo(string? Addr, long? PrefixLen);

public record EndpointIpamConfiguration(
    string? Ipv4Address,
    string? Ipv6Address,
    IReadOnlyList<string> LinkLocalIPs);

public record EndpointSettingsInfo(
    EndpointIpamConfiguration? IpamConfig,
    IReadOnlyList<string> Links,
    string? MacAddress,
    IReadOnlyList<string> Aliases,
    string? NetworkID,
    string? EndpointID,
    string? Gateway,
    string? IpAddress,
    long? IpPrefixLen,
    string? Ipv6Gateway,
    string? GlobalIPv6Address,
    long? GlobalIPv6PrefixLen,
    IReadOnlyDictionary<string, string> DriverOpts,
    IReadOnlyList<string> DNSNames);

public record NetworkSettingsInfo(
    string? Bridge,
    string? SandboxID,
    bool? HairpinMode,
    string? LinkLocalIPv6Address,
    long? LinkLocalIPv6PrefixLen,
    string? SandboxKey,
    IReadOnlyList<IpAddressInfo> SecondaryIPAddresses,
    IReadOnlyList<IpAddressInfo> SecondaryIPv6Addresses,
    string? EndpointID,
    string? Gateway,
    string? GlobalIPv6Address,
    long? GlobalIPv6PrefixLen,
    string? IpAddress,
    long? IpPrefixLen,
    string? Ipv6Gateway,
    string? MacAddress,
    IDictionary<string, IReadOnlyList<HostPortBinding>> Ports,
    IReadOnlyDictionary<string, EndpointSettingsInfo> Networks);