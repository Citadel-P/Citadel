using Citadel.Agent.Common.V1;
using Citadel.Agent.Containers.V1;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Google.Protobuf.Collections;

namespace Infrastructure.Connectors.Mappers;

internal static class ContainerMapper
{
    public static ContainerInspectionInfo Map(this InspectContainerResponse response) 
        => new 
        (
            Id: response.Id,
            Created: response.Created,
            Path: response.Path,
            Args: response.Args?.ToList() ?? [],
            State: response.State.Map(),
            Image: response.Image,
            ResolvConfPath: response.ResolvConfPath,
            HostnamePath: response.HostnamePath,
            HostsPath: response.HostsPath,
            LogPath: response.LogPath,
            Name: response.Name,
            RestartCount: response.RestartCount,
            Driver: response.Driver,
            Platform: response.Platform,
            MountLabel: response.MountLabel,
            ProcessLabel: response.ProcessLabel,
            AppArmorProfile: response.AppArmorProfile,
            ExecIDs: response.ExecIDs?.ToList() ?? [],
            HostConfig: response.HostConfig?.Map(),
            GraphDriver: response.GraphDriver?.Map(),
            SizeRw: response.SizeRw,
            SizeRootFs: response.SizeRootFs,
            Mounts: response.Mounts?.Select(m => m.Map()).ToList() ?? [],
            Config: response.Config?.Map(),
            NetworkSettings: response.NetworkSettings?.Map()
        );

    private static ContainerRuntimeState Map(this ContainerState state)
        => new 
        (
            Status: state.Status.Map(),
            Running: state.Running,
            Paused: state.Paused,
            Restarting: state.Restarting,
            OOMKilled: state.OOMKilled,
            Dead: state.Dead,
            Pid: state.Pid,
            ExitCode: state.ExitCode,
            Error: state.Error,
            StartedAt: state.StartedAt,
            FinishedAt: state.FinishedAt,
            Health: state.Health is not null ? new ContainerHealthStatus(state.Health.Status, state.Health.FailingStreak) : null
        );

    private static HostConfiguration Map(this HostConfig config)
        => new
        (
            NetworkMode: config.NetworkMode,
            RestartPolicy: new Domain.Contracts.Resources.Containers.RestartPolicy(config.RestartPolicy.Name, config.RestartPolicy.MaximumRetryCount),
            AutoRemove: config.AutoRemove,
            Privileged: config.Privileged,
            PublishAllPorts: config.PublishAllPorts,
            ReadonlyRootfs: config.ReadonlyRootfs,
            Dns: config.Dns?.ToList() ?? [],
            DnsOptions: config.DnsOptions?.ToList() ?? [],
            DnsSearch: config.DnsSearch?.ToList() ?? [],
            ExtraHosts: config.ExtraHosts?.ToList() ?? [],
            VolumesFrom: config.VolumesFrom?.ToList() ?? [],
            CapAdd: config.CapAdd?.ToList() ?? [],
            CapDrop: config.CapDrop?.ToList() ?? [],
            SecurityOpt: config.SecurityOpt?.ToList() ?? [],
            StorageOpt: config.StorageOpt?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? [],
            CgroupnsMode: config.CgroupnsMode,
            ShmSize: config.ShmSize,
            Tmpfs: config.Tmpfs?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? [],
            Sysctls: config.Sysctls?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? [],
            LogConfig: new LogConfiguration(config.LogConfig.Type, config.LogConfig.Config),
            Binds: config.Binds,
            ContainerIDFile: config.ContainerIDFile,
            PortBindings: config.PortBindings.Map(),
            VolumeDriver: config.VolumeDriver,
            Mounts: config.Mounts?.Map() ?? [],
            ConsoleSize: config.ConsoleSize,
            Annotations: config.Annotations,
            GroupAdd: config.GroupAdd,
            IpcMode: config.IpcMode,
            Cgroup: config.Cgroup,
            Links: config.Links,
            OomScoreAdj: config.OomScoreAdj,
            PidMode: config.PidMode,
            UtsMode: config.UTSMode,
            UsernsMode: config.UsernsMode,
            Runtime: config.Runtime,
            Isolation: config.Isolation,
            MaskedPaths: config.MaskedPaths,
            ReadonlyPaths: config.ReadonlyPaths,
            MemorySwap: config.MemorySwap,
            MemorySwappiness: config.MemorySwappiness,
            MemoryReservation: config.MemoryReservation,
            KernelMemoryTCP: config.KernelMemoryTCP,
            NanoCpus: config.NanoCpus,
            PidsLimit: config.PidsLimit,
            Memory: config.Memory,
            IoMaximumBandwidth: config.IOMaximumBandwidth,
            CpuPeriod: config.CpuPeriod,
            CpuCount: config.CpuCount,
            CpuPercent: config.CpuPercent,
            Ulimits: config.Ulimits?.Map() ?? []
        );

    private static IReadOnlyList<Ulimit> Map(this RepeatedField<Ulimits> ulimits)
        => [.. ulimits.Select(Map)];

    private static Ulimit Map(this Ulimits ulimits)
        => new 
        (
            Name: ulimits.Name,
            Soft: ulimits.Soft, 
            Hard: ulimits.Hard
        );

    private static IReadOnlyList<HostMount> Map(this RepeatedField<Mount> mounts)
        => [.. mounts.Select(Map)];

    private static HostMount Map(this Mount mount)
        => new 
        (
            Target: mount.Target,
            Source: mount.Source,
            Type: mount.Type,
            ReadOnly: mount.ReadOnly,
            Consistency: mount.Consistency,
            BindOptions : mount.BindOptions?.Map(),
            VolumeOptions: mount.VolumeOptions?.Map()
        );

    private static Domain.Contracts.Resources.Containers.BindOptions Map(this Citadel.Agent.Common.V1.BindOptions bindOptions)
        => new 
        (
             Propagation: bindOptions.Propagation,
             NonRecursive: bindOptions.NonRecursive,
             CreateMountpoint: bindOptions.CreateMountpoint,
             ReadOnlyNonRecursive: bindOptions.ReadOnlyNonRecursive,
             ReadOnlyForceRecursive: bindOptions.ReadOnlyForceRecursive
        );

    private static Domain.Contracts.Resources.Containers.VolumeOptions Map(this Citadel.Agent.Common.V1.VolumeOptions volumeOptions)
        => new
        (
            NoCopy: volumeOptions.NoCopy,
            Labels: volumeOptions.Labels,
            DriverConfig: volumeOptions.DriverConfig?.Map(),
            Subpath: volumeOptions.Subpath
        );

    private static DriverConfiguration Map(this DriverConfig driverConfig)
        => new
        (
            Name: driverConfig.Name,
            Options: driverConfig.Options
        );

    private static GraphDriverDataInfo Map(this GraphDriverData graphDriver)
        => new
        (
            Name: graphDriver.Name,
            Data: graphDriver.Data?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
        );

    private static MountPointInfo Map(this MountPoint mount)
        => new
        (
            Type: mount.Type,
            Name: mount.Name,
            Source: mount.Source,
            Destination: mount.Destination,
            Driver: mount.Driver,
            Mode: mount.Mode,
            RW: mount.RW,
            Propagation: mount.Propagation
        );

    private static ContainerConfiguration Map(this ContainerConfig config)
        => new
        (
            Hostname: config.Hostname,
            Domainname: config.Domainname,
            User: config.User,
            AttachStdin: config.AttachStdin,
            AttachStdout: config.AttachStdout,
            AttachStderr: config.AttachStderr,
            ExposedPorts: config.ExposedPorts?.ToList(),
            Tty: config.Tty,
            NetworkDisabled: config.NetworkDisabled,
            MacAddress: config.MacAddress,
            OpenStdin: config.OpenStdin,
            StdinOnce: config.StdinOnce,
            Env: config.Env?.ToList() ?? [],
            Cmd: config.Cmd?.ToList() ?? [],
            Image: config.Image,
            Volumes: config.Volumes?.ToList(),
            WorkingDir: config.WorkingDir,
            Entrypoint: config.Entrypoint?.ToList() ?? [],
            OnBuild: config.OnBuild?.ToList() ?? [],
            Labels: config.Labels?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
        );

    private static NetworkSettingsInfo Map(this NetworkSettings settings)
        => new(
            Bridge: settings.Bridge,
            SandboxID: settings.SandboxID,
            HairpinMode: settings.HairpinMode,
            LinkLocalIPv6Address: settings.LinkLocalIPv6Address,
            LinkLocalIPv6PrefixLen: settings.LinkLocalIPv6PrefixLen,
            Ports: settings.Ports?.Map() ?? [],
            EndpointID: settings.EndpointID,
            Gateway: settings.Gateway,
            IpAddress: settings.IpAddress,
            IpPrefixLen: settings.IpPrefixLen,
            Ipv6Gateway: settings.Ipv6Gateway,
            MacAddress: settings.MacAddress,
            GlobalIPv6Address: settings.GlobalIPv6Address,
            GlobalIPv6PrefixLen: settings.GlobalIPv6PrefixLen,
            SandboxKey: settings.SandboxKey,
            Networks: settings.Networks?.ToDictionary(kv => kv.Key, kv => kv.Value.Map()) ?? [],
            SecondaryIPAddresses: settings.SecondaryIPAddresses?.Select(ip => ip.Map()).ToList() ?? [],
            SecondaryIPv6Addresses: settings.SecondaryIPv6Addresses?.Select(ip => ip.Map()).ToList() ?? []
        );

    public static IpAddressInfo Map(this Address address)
        => new
        (
                Addr: address.Addr,
                PrefixLen: address.PrefixLen
        );

    private static IReadOnlyList<IDictionary<string, IReadOnlyList<HostPortBinding>>> Map(this RepeatedField<MapFieldPortBinding> binding)
    {
        if (binding is null || binding.Count == 0)
            return [];

        var result = new List<IDictionary<string, IReadOnlyList<HostPortBinding>>>(binding.Count);

        foreach (var mapField in binding)
        {
            var dict = new Dictionary<string, IReadOnlyList<HostPortBinding>>(1);
            var bindings = mapField.Value?.Select(Map).ToList() ?? [];
            dict[mapField.Key] = bindings;
            result.Add(dict);
        }

        return result;
    }

    private static HostPortBinding Map(this PortBinding binding)
        => new(
            HostIP: binding.HostIP,
            HostPort: binding.HostPort
        );

    private static EndpointSettingsInfo Map(this EndpointSettings endpoint)
        => new(
            IpamConfig: endpoint.IPAMConfig?.Map(),
            Links: endpoint.Links?.ToList() ?? [],
            Aliases: endpoint.Aliases?.ToList() ?? [],
            NetworkID: endpoint.NetworkID,
            EndpointID: endpoint.EndpointID,
            Gateway: endpoint.Gateway,
            IpAddress: endpoint.IpAddress,
            IpPrefixLen: endpoint.IpPrefixLen,
            Ipv6Gateway: endpoint.Ipv6Gateway,
            GlobalIPv6Address: endpoint.GlobalIPv6Address,
            GlobalIPv6PrefixLen: endpoint.GlobalIPv6PrefixLen,
            MacAddress: endpoint.MacAddress,
            DNSNames: endpoint.DNSNames,
            DriverOpts: endpoint.DriverOpts?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
        );

    private static EndpointIpamConfiguration Map(this EndpointIPAMConfig endpointIPAMConfig)
        => new
        (
            Ipv4Address: endpointIPAMConfig.Ipv4Address,
            Ipv6Address: endpointIPAMConfig.Ipv6Address,
            LinkLocalIPs: endpointIPAMConfig.LinkLocalIPs
        );


    public static Dictionary<string, Container> Map(this ListContainersResponse response, Guid platformId)
        => response.Containers.ToDictionary(
            pair => pair.Key,
            pair => pair.Value.Map(platformId));

    private static Container Map(this ContainerMessage container, Guid platformId)
    {
        var result = new Container
        (
            name: container.Name,
            image: container.Image,
            stack: container.Stack,
            platformId: platformId,
            containerId: container.Id,
            created: container.Created,
            command: container.Command,
            state: container.State.Map(),
            ports: container.Ports?.Map()?.ToList() ?? []
        );

        if (container.ContainerStatMessage != null)
        {
            result.AppendStat(container.ContainerStatMessage.Map());
        }
        return result;
    }

    public static ContainerStat Map(this ContainerStatMessage statMessage)
        => new (
            memoryUsage: statMessage.MemoryUsage,
            memoryLimit: statMessage.MemoryLimit,
            cpuUsage: statMessage.CpuUsage,
            rxBytes: statMessage.RxBytes,
            txBytes: statMessage.TxBytes,
            created: 0,
            containerId: Guid.Empty
        );

    public static IEnumerable<ContainerPort> Map(this IEnumerable<PortMessage> ports)
        => ports.Select(Map);

    private static ContainerPort Map(this PortMessage port) 
        => new (
            IP: port.IP,
            PrivatePort: port.PrivatePort,
            PublicPort: port.PublicPort,
            Type: port.Type
        );

    public static MapField<string, OptionChain> Map(this IDictionary<string, IDictionary<string, bool>> filters)
    {
        var map = new MapField<string, OptionChain>();
        foreach (var pair in filters)
        {
            map.Add(pair.Key, pair.Value.Map());
        }
        return map;
    }

    private static OptionChain Map(this IDictionary<string, bool>? options)
    {
        var optionChain = new OptionChain();
        if (options != null)
        {
            foreach (var (option, enabled) in options)
            {
                optionChain.Options.Add(option, enabled);
            }
        }
        return optionChain;
    }

    public static ContainerLogInfo Map(this ContainerLogResponse logInfo)
        => new(Log: logInfo.Log);

    public static ContainerStats Map(this ContainerStatsResponse statsResponse)
        => new (Containers: statsResponse.Containers.ToDictionary(c => c.Key, c => c.Value.Map()));

    public static DaemonEventInfo Map(this DaemonEventResponse response, Guid platformId)
        => new
        (
            Id: response.Id,
            Action: response.Action,
            ContainerId: response.ContainerId,
            Type: response.EventMessageType.Map(),
            Container: response.Container?.Map(platformId)
        );

    public static ContainerEventType Map(this EventMessageType type)
        => type switch
        {
            EventMessageType.Builder => ContainerEventType.Builder,
            EventMessageType.Config => ContainerEventType.Config,
            EventMessageType.Container => ContainerEventType.Container,
            EventMessageType.Daemon => ContainerEventType.Daemon,
            EventMessageType.Image => ContainerEventType.Image,
            EventMessageType.Network => ContainerEventType.Network,
            EventMessageType.Node => ContainerEventType.Node,
            EventMessageType.Plugin => ContainerEventType.Plugin,
            EventMessageType.Secret => ContainerEventType.Secret,
            EventMessageType.Service => ContainerEventType.Service,
            EventMessageType.Volume => ContainerEventType.Volume,
            _ => ContainerEventType.Unknown
        };

    public static ContainerStateStatus Map(this ContainerStateType state)
        => state switch
        {
            ContainerStateType.Unknown => ContainerStateStatus.Unknown,
            ContainerStateType.Running => ContainerStateStatus.Running,
            ContainerStateType.Paused => ContainerStateStatus.Paused,
            ContainerStateType.Restarting => ContainerStateStatus.Restarting,
            ContainerStateType.Dead => ContainerStateStatus.Dead,
            ContainerStateType.Exited => ContainerStateStatus.Exited,
            ContainerStateType.Removing => ContainerStateStatus.Removing,
            _ => ContainerStateStatus.Unknown,
        };
}