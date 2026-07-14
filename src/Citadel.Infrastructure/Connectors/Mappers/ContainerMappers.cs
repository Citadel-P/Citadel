using Citadel.SharedModels.V1;
using Citadel.Containers.V1;
using Citadel.Platforms.V1;
using Domain;
using Domain.Contracts.Resources.Containers;
using Google.Protobuf.Collections;
using Hosting.DockerClient.Models.Containers;

namespace Infrastructure.Connectors.Mappers;

internal static class ContainerMappers
{
    internal static ContainerInspectionInfo Map(this InspectContainerResponse container) 
        => new 
        (
            Id: container.Id,
            Created: container.Created,
            Path: container.Path,
            Args: container.Args?.ToList() ?? [],
            State: container.State.Map(),
            Image: container.Image,
            ResolvConfPath: container.ResolvConfPath,
            HostnamePath: container.HostnamePath,
            HostsPath: container.HostsPath,
            LogPath: container.LogPath,
            Name: container.Name,
            RestartCount: container.RestartCount,
            Driver: container.Driver,
            Platform: container.Platform,
            MountLabel: container.MountLabel,
            ProcessLabel: container.ProcessLabel,
            AppArmorProfile: container.AppArmorProfile,
            ExecIDs: container.ExecIDs?.ToList() ?? [],
            HostConfig: container.HostConfig?.Map(),
            GraphDriver: container.GraphDriver?.Map(),
            SizeRw: container.SizeRw,
            SizeRootFs: container.SizeRootFs,
            Mounts: container.Mounts?.Select(m => m.Map()).ToList() ?? [],
            Config: container.Config?.Map(),
            NetworkSettings: container.NetworkSettings?.Map()
        );

    internal static ContainerInspectionInfo Map(this Hosting.DockerClient.ContainerInspectResponse container)
        => new
        (
            Id: container.Id,
            Created: container.Created,
            Path: container.Path,
            Args: container.Args?.ToList() ?? [],
            State: container.State?.Map(),
            Image: container.Image,
            ResolvConfPath: container.ResolvConfPath,
            HostnamePath: container.HostnamePath,
            HostsPath: container.HostsPath,
            LogPath: container.LogPath,
            Name: container.Name,
            RestartCount: container.RestartCount,
            Driver: container.Driver,
            Platform: container.Platform,
            MountLabel: container.MountLabel,
            ProcessLabel: container.ProcessLabel,
            AppArmorProfile: container.AppArmorProfile,
            ExecIDs: container.ExecIDs?.ToList() ?? [],
            HostConfig: container.HostConfig?.Map(),
            GraphDriver: container.GraphDriver?.Map(),
            SizeRw: container.SizeRw,
            SizeRootFs: container.SizeRootFs,
            Mounts: container.Mounts?.Select(m => m.Map()).ToList() ?? [],
            Config: container.Config?.Map(),
            NetworkSettings: container.NetworkSettings?.Map()
        );

    private static NetworkSettingsInfo Map(this Hosting.DockerClient.NetworkSettings settings)
       => new(
           Bridge: settings.Bridge,
           SandboxID: settings.SandboxID,
           HairpinMode: settings.HairpinMode,
           LinkLocalIPv6Address: settings.LinkLocalIPv6Address,
           LinkLocalIPv6PrefixLen: settings.LinkLocalIPv6PrefixLen,
           Ports: settings.Ports?.Map() ?? [],
           EndpointID: settings.EndpointID,
           Gateway: settings.Gateway,
           IpAddress: settings.IPAddress,
           IpPrefixLen: settings.IPPrefixLen,
           Ipv6Gateway: settings.IPv6Gateway,
           MacAddress: settings.MacAddress,
           GlobalIPv6Address: settings.GlobalIPv6Address,
           GlobalIPv6PrefixLen: settings.GlobalIPv6PrefixLen,
           SandboxKey: settings.SandboxKey,
           Networks: settings.Networks?.ToDictionary(kv => kv.Key, kv => kv.Value.Map()) ?? [],
           SecondaryIPAddresses: settings.SecondaryIPAddresses?.Select(s => new IpAddressInfo(Addr: s.Addr, PrefixLen: s.PrefixLen) ).ToList() ?? [],
           SecondaryIPv6Addresses: settings.SecondaryIPv6Addresses?.Select(s => new IpAddressInfo(Addr: s.Addr, PrefixLen: s.PrefixLen)).ToList() ?? []
       );

    private static EndpointSettingsInfo Map(this Hosting.DockerClient.EndpointSettings endpoint)
       => new(
           IpamConfig: endpoint.IPAMConfig?.Map(),
           Links: endpoint.Links?.ToList() ?? [],
           Aliases: endpoint.Aliases?.ToList() ?? [],
           NetworkID: endpoint.NetworkID,
           EndpointID: endpoint.EndpointID,
           Gateway: endpoint.Gateway,
           IpAddress: endpoint.IPAddress,
           IpPrefixLen: endpoint.IPPrefixLen,
           Ipv6Gateway: endpoint.IPv6Gateway,
           GlobalIPv6Address: endpoint.GlobalIPv6Address,
           GlobalIPv6PrefixLen: endpoint.GlobalIPv6PrefixLen,
           MacAddress: endpoint.MacAddress,
           DNSNames: endpoint.DNSNames?.ToList() ?? [],
           DriverOpts: endpoint.DriverOpts?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
       );

    private static EndpointIpamConfiguration Map(this Hosting.DockerClient.EndpointIPAMConfig endpointIPAMConfig)
        => new
        (
            Ipv4Address: endpointIPAMConfig.IPv4Address,
            Ipv6Address: endpointIPAMConfig.IPv6Address,
            LinkLocalIPs: endpointIPAMConfig.LinkLocalIPs?.ToList() ?? []
        );

    private static ContainerConfiguration Map(this Hosting.DockerClient.ContainerConfig config)
        => new
        (
            Hostname: config.Hostname,
            Domainname: config.Domainname,
            User: config.User,
            AttachStdin: config.AttachStdin,
            AttachStdout: config.AttachStdout,
            AttachStderr: config.AttachStderr,
            Tty: config.Tty,
            NetworkDisabled: config.NetworkDisabled,
            MacAddress: config.MacAddress,
            OpenStdin: config.OpenStdin,
            StdinOnce: config.StdinOnce,
            Env: config.Env?.ToList() ?? [],
            Cmd: config.Cmd?.ToList() ?? [],
            Image: config.Image,
            Volumes: config.Volumes?.Select(s => s.Key).ToList(),
            WorkingDir: config.WorkingDir,
            OnBuild: config.OnBuild?.ToList() ?? [],
            Entrypoint: config.Entrypoint?.ToList() ?? [],
            ExposedPorts: config.ExposedPorts?.Select(s => s.Key).ToList(),
            Labels: config.Labels?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
        );

    private static MountPointInfo Map(this Hosting.DockerClient.MountPoint mount)
        => new
        (
            Name: mount.Name,
            Source: mount.Source,
            Type: mount.Type?.ToString(),
            Destination: mount.Destination,
            Driver: mount.Driver,
            Mode: mount.Mode,
            RW: mount.RW,
            Propagation: mount.Propagation
        );

    private static HostConfiguration Map(this Hosting.DockerClient.HostConfig config)
    {
        return new 
            (
                NetworkMode: config.NetworkMode,
                RestartPolicy: new Domain.Contracts.Resources.Containers.RestartPolicy(config.RestartPolicy?.Name.ToString(), config.RestartPolicy?.MaximumRetryCount),
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
                CgroupnsMode: config.CgroupnsMode?.ToString(),
                ShmSize: config.ShmSize ?? 0,
                Tmpfs: config.Tmpfs?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? [],
                Sysctls: config.Sysctls?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? [],
                LogConfig: new LogConfiguration(config.LogConfig?.Type?.ToString(), config.LogConfig?.Config?.ToDictionary() ?? []),
                Binds: config.Binds?.ToList() ?? [],
                ContainerIDFile: config.ContainerIDFile,
                PortBindings: config.PortBindings?.Map() ?? [],
                VolumeDriver: config.VolumeDriver,
                Mounts: config.Mounts?.Map() ?? [],
                ConsoleSize: config.ConsoleSize?.ToList() ?? [],
                Annotations: config.Annotations?.ToDictionary() ?? [],
                GroupAdd: config.GroupAdd?.ToList() ?? [],
                IpcMode: config.IpcMode,
                Cgroup: config.Cgroup,
                Links: config.Links?.ToList() ?? [],
                OomScoreAdj: config.OomScoreAdj,
                PidMode: config.PidMode,
                UtsMode: config.UTSMode,
                UsernsMode: config.UsernsMode,
                Runtime: config.Runtime,
                Isolation: config.Isolation?.ToString(),
                MaskedPaths: config.MaskedPaths?.ToList() ?? [],
                ReadonlyPaths: config.ReadonlyPaths?.ToList() ?? [],
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
    }
    
    private static IReadOnlyList<HostMount> Map(this ICollection<Hosting.DockerClient.Mount> mounts)
        => [.. mounts.Select(Map)];

    private static HostMount Map(this Hosting.DockerClient.Mount mount)
        => new
        (
            Target: mount.Target,
            Source: mount.Source,
            Type: mount.Type?.ToString(),
            ReadOnly: mount.ReadOnly,
            Consistency: mount.Consistency,
            BindOptions: mount.BindOptions?.Map(),
            VolumeOptions: mount.VolumeOptions?.Map()
        );

    private static Domain.Contracts.Resources.Containers.BindOptions Map(this Hosting.DockerClient.BindOptions bindOptions)
        => new
        (
             NonRecursive: bindOptions.NonRecursive,
             CreateMountpoint: bindOptions.CreateMountpoint,
             Propagation: bindOptions.Propagation?.ToString(),
             ReadOnlyNonRecursive: bindOptions.ReadOnlyNonRecursive,
             ReadOnlyForceRecursive: bindOptions.ReadOnlyForceRecursive
        );

    private static Domain.Contracts.Resources.Containers.VolumeOptions Map(this Hosting.DockerClient.VolumeOptions volumeOptions)
        => new
        (
            NoCopy: volumeOptions.NoCopy,
            Subpath: volumeOptions.Subpath,
            Labels: volumeOptions.Labels.ToDictionary(),
            DriverConfig: volumeOptions.DriverConfig?.Map()
        );

    private static DriverConfiguration Map(this Hosting.DockerClient.DriverConfig driverConfig)
       => new
       (
           Name: driverConfig.Name,
           Options: driverConfig.Options?.ToDictionary() ?? []
       );

    private static IReadOnlyList<Ulimit> Map(this ICollection<Hosting.DockerClient.Ulimits> ulimits)
        => [.. ulimits.Select(Map)];

    private static Ulimit Map(this Hosting.DockerClient.Ulimits ulimits)
        => new
        (
            Name: ulimits.Name,
            Soft: ulimits.Soft,
            Hard: ulimits.Hard
        );

    private static GraphDriverDataInfo Map(this Hosting.DockerClient.DriverData graphDriver)
       => new
       (
           Name: graphDriver.Name,
           Data: graphDriver.Data?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
       );

    private static Dictionary<string, IReadOnlyList<Domain.Entities.HostPortBinding>> Map(this Hosting.DockerClient.PortMap portMap)
    {
        if (portMap is null || portMap.Count == 0)
            return [];

        var result = new Dictionary<string, IReadOnlyList<Domain.Entities.HostPortBinding>>(portMap.Count);

        foreach (var mapField in portMap)
        {
            var bindings = mapField.Value?.Select(s => new Domain.Entities.HostPortBinding(s.HostIp, s.HostPort)).ToList() ?? [];
            result[mapField.Key] = bindings;
        }

        return result;
    }

    private static ContainerRuntimeState Map(this Hosting.DockerClient.ContainerState state)
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
            Health: state.Health is not null ? new ContainerHealthStatus(state.Health?.Status?.ToString(), state.Health?.FailingStreak) : null
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
            PortBindings: config.PortBindings?.Map(),
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

    private static Domain.Contracts.Resources.Containers.BindOptions Map(this Citadel.SharedModels.V1.BindOptions bindOptions)
        => new 
        (
             Propagation: bindOptions.Propagation,
             NonRecursive: bindOptions.NonRecursive,
             CreateMountpoint: bindOptions.CreateMountpoint,
             ReadOnlyNonRecursive: bindOptions.ReadOnlyNonRecursive,
             ReadOnlyForceRecursive: bindOptions.ReadOnlyForceRecursive
        );

    private static Domain.Contracts.Resources.Containers.VolumeOptions Map(this Citadel.SharedModels.V1.VolumeOptions volumeOptions)
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

    internal static IpAddressInfo Map(this Address address)
        => new
        (
                Addr: address.Addr,
                PrefixLen: address.PrefixLen
        );

    internal static Dictionary<string, IReadOnlyList<Domain.Entities.HostPortBinding>> Map(this MapField<string, HostPortBindingList> bindings)
    {
        if (bindings is null || bindings.Count == 0)
            return [];

        var result = new Dictionary<string, IReadOnlyList<Domain.Entities.HostPortBinding>>(bindings.Count);

        foreach (var mapField in bindings)
        {
            result[mapField.Key] = mapField.Value?.Map() ?? [];
        }
        return result;
    }

    private static List<Domain.Entities.HostPortBinding> Map(this HostPortBindingList items)
    {
        var result = new List<Domain.Entities.HostPortBinding>();
        foreach (var binding in items.HostPortBinding)
        {
            result.Add(new Domain.Entities.HostPortBinding(binding.HostIP, binding.HostPort));
        }
        return result;
    }

    private static EndpointSettingsInfo Map(this Citadel.SharedModels.V1.EndpointSettings endpointSettings)
        => new(
            IpamConfig: endpointSettings.IPAMConfig?.Map(),
            Links: endpointSettings.Links?.ToList() ?? [],
            Aliases: endpointSettings.Aliases?.ToList() ?? [],
            NetworkID: endpointSettings.NetworkID,
            EndpointID: endpointSettings.EndpointID,
            Gateway: endpointSettings.Gateway,
            IpAddress: endpointSettings.IpAddress,
            IpPrefixLen: endpointSettings.IpPrefixLen,
            Ipv6Gateway: endpointSettings.Ipv6Gateway,
            GlobalIPv6Address: endpointSettings.GlobalIPv6Address,
            GlobalIPv6PrefixLen: endpointSettings.GlobalIPv6PrefixLen,
            MacAddress: endpointSettings.MacAddress,
            DNSNames: endpointSettings.DNSNames,
            DriverOpts: endpointSettings.DriverOpts?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
        );

    private static EndpointIpamConfiguration Map(this EndpointIPAMConfig endpointIPAMConfig)
        => new
        (
            Ipv4Address: endpointIPAMConfig.Ipv4Address,
            Ipv6Address: endpointIPAMConfig.Ipv6Address,
            LinkLocalIPs: endpointIPAMConfig.LinkLocalIPs
        );

    internal static Dictionary<string, DockerContainer> Map(this ListContainersResponse response)
        => response.Containers.ToDictionary(
            pair => pair.Key,
            pair => pair.Value.Map());

    internal static DockerContainer Map(this ContainerMessage container)
        => new
        (
            Name: container.Name,
            Image: container.Image,
            ImageId: container.ImageID,
            Stack: container.Stack,
            Id: container.Id,
            Created: container.Created,
            State: container.State.Map(),
            ContainerStat: container.ContainerStatMessage?.Map(),
            Ports: container.Ports?.Map()
        );

    internal static DockerContainerStat Map(this ContainerStatMessage statMessage)
        => new (
            MemoryActive: statMessage.MemoryActive,
            MemoryCache: statMessage.MemoryCache,
            MemoryLimit: statMessage.MemoryLimit,
            CpuUsage: statMessage.CpuUsage,
            RxBytes: statMessage.RxBytes,
            TxBytes: statMessage.TxBytes
        );

    internal static MapField<string, OptionChain> Map(this IDictionary<string, IDictionary<string, bool>> filters)
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

    internal static DaemonEventInfo Map(this DaemonEventResponse @event)
    {
        return @event.KindCase switch
        {
            DaemonEventResponse.KindOneofCase.DaemonContainerEventResponse
                => MapContainer(@event.DaemonContainerEventResponse),

            DaemonEventResponse.KindOneofCase.DaemonImageEventResponse
                => MapImage(@event.DaemonImageEventResponse),

            DaemonEventResponse.KindOneofCase.DaemonVolumeEventResponse
                => MapVolume(@event.DaemonVolumeEventResponse),

            DaemonEventResponse.KindOneofCase.DaemonNetworkEventResponse
                => MapNetwork(@event.DaemonNetworkEventResponse),

            _ => throw new InvalidOperationException("Unknown event kind")
        };
    }

    private static DaemonContainerEventInfo MapContainer(DaemonContainerEventResponse evt)
    {
        return new DaemonContainerEventInfo
        (
            Action: evt.Action,
            ContainerId: evt.ContainerId,
            Container: evt.Container?.Map()
        );
    }

    private static DaemonImageEventInfo MapImage(DaemonImageEventResponse evt)
    {
        return new DaemonImageEventInfo
        (
            Action: evt.Action,
            ImageId: evt.ImageId,
            Image: evt.Image?.Map()
        );
    }

    private static DaemonVolumeEventInfo MapVolume(DaemonVolumeEventResponse evt)
    {
        return new DaemonVolumeEventInfo
        (
            Action: evt.Action,
            VolumeId: evt.VolumeId,
            Volume: evt.Volume?.Map()
        );
    }

    private static DaemonNetworkEventInfo MapNetwork(DaemonNetworkEventResponse evt)
    {
        return new DaemonNetworkEventInfo
        (
            Action: evt.Action,
            NetworkId: evt.NetworkId,
            Network: evt.Network?.Map()
        );
    }

    internal static IReadOnlyDictionary<string, DockerContainer> Map(this IReadOnlyDictionary<string, ContainerResult> containers) 
        => containers.ToDictionary(c => c.Key, c => c.Value.Map());

    internal static DockerContainer Map(this ContainerResult container)
        => new
        (
            Name: container?.Name ?? "",
            Image: container?.Image ?? "",
            ImageId: container?.ImageID ?? "",
            Stack: container?.Stack,
            Id: container?.Id ?? "",
            Created: container?.Created,
            ContainerStat: container?.ContainerStat?.Map(),
            Ports: container?.Ports?.Map(),
            State: container?.State?.Map() ?? ContainerStateStatus.Unknown
        );

    internal static Dictionary<string, IReadOnlyList<Domain.Entities.HostPortBinding>> Map(this IDictionary<string, IReadOnlyList<Hosting.DockerClient.PortBinding>> bindings)
    {
        if (bindings is null || bindings.Count == 0)
            return [];

        var result = new Dictionary<string, IReadOnlyList<Domain.Entities.HostPortBinding>>(bindings.Count);

        foreach (var mapField in bindings)
        {
            result[mapField.Key] = mapField.Value?.Map() ?? [];
        }
        return result;
    }

    private static List<Domain.Entities.HostPortBinding> Map(this IReadOnlyList<Hosting.DockerClient.PortBinding> items)
    {
        var result = new List<Domain.Entities.HostPortBinding>();
        foreach (var binding in items)
        {
            result.Add(new Domain.Entities.HostPortBinding(binding.HostIp, binding.HostPort));
        }
        return result;
    }

    private static DockerContainerStat Map(this ContainerStatResult stats)
        => new
        (
            MemoryActive: stats.MemoryActive,
            MemoryCache: stats.MemoryCache,
            CpuUsage: stats.CpuUsage,
            MemoryLimit: stats.MemoryLimit,
            RxBytes: stats.RxBytes,
            TxBytes: stats.TxBytes
        );

    internal static Dictionary<string, DockerContainerStat> Map(this Dictionary<string, ContainerStatResult> result)
        => result.ToDictionary(kv => kv.Key, kv => kv.Value.Map());

    internal static Dictionary<string, DockerContainerStat> Map(this MapField<string, ContainerStatMessage> result)
        => result.ToDictionary(kv => kv.Key, kv => kv.Value.Map());

    internal static Hosting.DockerClient.EndpointSettings Map(this Domain.Contracts.Resources.Networks.EndpointSettings endPointsConfig)
    {
        return new Hosting.DockerClient.EndpointSettings()
        {
            IPAMConfig = endPointsConfig.IPAMConfig?.Map(),
            Links = endPointsConfig.Links?.ToList() ?? [],
            NetworkID = endPointsConfig.NetworkID,
            EndpointID = endPointsConfig.EndpointID,
            Gateway = endPointsConfig.Gateway,
            IPAddress = endPointsConfig.IPAddress,
            IPPrefixLen = endPointsConfig.IPPrefixLen,
            IPv6Gateway = endPointsConfig.IPv6Gateway,
            GlobalIPv6Address = endPointsConfig.GlobalIPv6Address,
            GlobalIPv6PrefixLen = endPointsConfig.GlobalIPv6PrefixLen,
            MacAddress = endPointsConfig.MacAddress,
            DNSNames = endPointsConfig.DNSNames?.ToList() ?? [],
            DriverOpts = endPointsConfig.DriverOpts?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? []
        };
    }

    internal static EndpointSettings MapAgent(this Domain.Contracts.Resources.Networks.EndpointSettings endPointsConfig)
    {
        return new EndpointSettings()
        {
            IPAMConfig = endPointsConfig.IPAMConfig?.MapAgent(),
            Links = { endPointsConfig.Links?.ToList() ?? [] },
            NetworkID = endPointsConfig.NetworkID,
            EndpointID = endPointsConfig.EndpointID,
            Gateway = endPointsConfig.Gateway,
            IpAddress = endPointsConfig.IPAddress,
            IpPrefixLen = endPointsConfig.IPPrefixLen,
            Ipv6Gateway = endPointsConfig.IPv6Gateway,
            GlobalIPv6Address = endPointsConfig.GlobalIPv6Address,
            GlobalIPv6PrefixLen = endPointsConfig.GlobalIPv6PrefixLen,
            MacAddress = endPointsConfig.MacAddress,
            DNSNames = { endPointsConfig.DNSNames?.ToList() ?? [] },
            DriverOpts = { endPointsConfig.DriverOpts?.ToDictionary(kv => kv.Key, kv => kv.Value) ?? [] }
        };
    }

    internal static Mount MapAgent(this HostMount mount)
        => new()
        {
            Target = mount.Target,
            Source = mount.Source,
            Type = mount.Type,
            ReadOnly = mount.ReadOnly,
            Consistency = mount.Consistency,
            BindOptions = mount.BindOptions?.MapAgent(),
            VolumeOptions = mount.VolumeOptions?.MapAgent()
        };

    private static Citadel.SharedModels.V1.BindOptions MapAgent(this Domain.Contracts.Resources.Containers.BindOptions options)
        => new()
        {
            Propagation = options.Propagation,
            NonRecursive = options.NonRecursive,
            CreateMountpoint = options.CreateMountpoint,
            ReadOnlyNonRecursive = options.ReadOnlyNonRecursive,
            ReadOnlyForceRecursive = options.ReadOnlyForceRecursive
        };

    private static Citadel.SharedModels.V1.VolumeOptions MapAgent(this Domain.Contracts.Resources.Containers.VolumeOptions options)
        => new()
        {
            NoCopy = options.NoCopy,
            Labels = { options.Labels.ToDictionary(kvp => kvp.Key, kvp => kvp.Value) },
            DriverConfig = options.DriverConfig?.MapAgent(),
            Subpath = options.Subpath
        };

    private static DriverConfig MapAgent(this DriverConfiguration driverConfig)
        => new()
        {
            Name = driverConfig.Name,
            Options = { driverConfig.Options.ToDictionary(kvp => kvp.Key, kvp => kvp.Value) }
        };

    internal static Hosting.DockerClient.EndpointIPAMConfig Map(this Domain.Contracts.Resources.Networks.EndpointIPAMConfig ipamConfig)
        => new Hosting.DockerClient.EndpointIPAMConfig
        {
            IPv4Address = ipamConfig.IPv4Address,
            IPv6Address = ipamConfig.IPv6Address,
            LinkLocalIPs = ipamConfig.LinkLocalIPs?.ToList() ?? []
        };

    internal static EndpointIPAMConfig MapAgent(this Domain.Contracts.Resources.Networks.EndpointIPAMConfig ipamConfig)
        => new EndpointIPAMConfig
        {
            Ipv4Address = ipamConfig.IPv4Address,
            Ipv6Address = ipamConfig.IPv6Address,
            LinkLocalIPs = { ipamConfig.LinkLocalIPs?.ToList() ?? [] }
        };

    internal static ContainerEventType Map(this EventMessageType type)
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

    internal static ContainerStateStatus Map(this ContainerStateType state)
        => state switch
        {
            ContainerStateType.Unknown => ContainerStateStatus.Unknown,
            ContainerStateType.Running => ContainerStateStatus.Running,
            ContainerStateType.Paused => ContainerStateStatus.Paused,
            ContainerStateType.Restarting => ContainerStateStatus.Restarting,
            ContainerStateType.Dead => ContainerStateStatus.Dead,
            ContainerStateType.Exited => ContainerStateStatus.Exited,
            ContainerStateType.Removing => ContainerStateStatus.Removing,
            ContainerStateType.Created => ContainerStateStatus.Created,
            _ => ContainerStateStatus.Unknown,
        };

    internal static ContainerStateStatus Map(this Hosting.DockerClient.ContainerSummaryState state)
        => state switch
        {
            Hosting.DockerClient.ContainerSummaryState.Running => ContainerStateStatus.Running,
            Hosting.DockerClient.ContainerSummaryState.Paused => ContainerStateStatus.Paused,
            Hosting.DockerClient.ContainerSummaryState.Restarting => ContainerStateStatus.Restarting,
            Hosting.DockerClient.ContainerSummaryState.Dead => ContainerStateStatus.Dead,
            Hosting.DockerClient.ContainerSummaryState.Exited => ContainerStateStatus.Exited,
            Hosting.DockerClient.ContainerSummaryState.Removing => ContainerStateStatus.Removing,
            Hosting.DockerClient.ContainerSummaryState.Created => ContainerStateStatus.Created,
            _ => ContainerStateStatus.Unknown,
        };

    internal static ContainerStateStatus Map(this Hosting.DockerClient.ContainerStateStatus? state)
        => state switch
        {
            Hosting.DockerClient.ContainerStateStatus.Running => ContainerStateStatus.Running,
            Hosting.DockerClient.ContainerStateStatus.Paused => ContainerStateStatus.Paused,
            Hosting.DockerClient.ContainerStateStatus.Restarting => ContainerStateStatus.Restarting,
            Hosting.DockerClient.ContainerStateStatus.Dead => ContainerStateStatus.Dead,
            Hosting.DockerClient.ContainerStateStatus.Exited => ContainerStateStatus.Exited,
            Hosting.DockerClient.ContainerStateStatus.Removing => ContainerStateStatus.Removing,
            Hosting.DockerClient.ContainerStateStatus.Created => ContainerStateStatus.Created,
            _ => ContainerStateStatus.Unknown,
        };

    internal static Hosting.DockerClient.Models.Containers.ContainerAction Map(this Domain.ContainerAction action)
        => action switch
        {
            Domain.ContainerAction.START => Hosting.DockerClient.Models.Containers.ContainerAction.START,
            Domain.ContainerAction.RESTART => Hosting.DockerClient.Models.Containers.ContainerAction.RESTART,
            Domain.ContainerAction.STOP => Hosting.DockerClient.Models.Containers.ContainerAction.STOP,
            Domain.ContainerAction.PAUSE => Hosting.DockerClient.Models.Containers.ContainerAction.PAUSE,
            Domain.ContainerAction.UNPAUSE => Hosting.DockerClient.Models.Containers.ContainerAction.UNPAUSE,
            _ => Hosting.DockerClient.Models.Containers.ContainerAction.STOP
        };

    internal static Hosting.DockerClient.RestartPolicyName Map(this ContainerRestartPolicy name)
        => name switch
        {
            ContainerRestartPolicy.No => Hosting.DockerClient.RestartPolicyName.No,
            ContainerRestartPolicy.Always => Hosting.DockerClient.RestartPolicyName.Always,
            ContainerRestartPolicy.OnFailure => Hosting.DockerClient.RestartPolicyName.OnFailure,
            ContainerRestartPolicy.UnlessStopped => Hosting.DockerClient.RestartPolicyName.UnlessStopped,
            _ => Hosting.DockerClient.RestartPolicyName.No
        };

    internal static Citadel.Containers.V1.RestartPolicy Map(this ContainerRestartPolicy? restartPolicy)
        => restartPolicy switch
        {
            ContainerRestartPolicy.No => Citadel.Containers.V1.RestartPolicy.No,
            ContainerRestartPolicy.Always => Citadel.Containers.V1.RestartPolicy.Always,
            ContainerRestartPolicy.OnFailure => Citadel.Containers.V1.RestartPolicy.OnFailure,
            ContainerRestartPolicy.UnlessStopped => Citadel.Containers.V1.RestartPolicy.UnlessStopped,
            _ => Citadel.Containers.V1.RestartPolicy.No
        };
}

internal sealed class LocalExecSessionAdapter : Domain.Contracts.Interfaces.IExecSession
{
    private readonly Hosting.DockerClient.Services.IExecSession _inner;

    public LocalExecSessionAdapter(Hosting.DockerClient.Services.IExecSession inner)
        => _inner = inner;

    public IAsyncEnumerable<ReadOnlyMemory<byte>> Output => _inner.Output;

    public Task SendAsync(ReadOnlyMemory<byte> input, CancellationToken ct)
        => _inner.SendAsync(input, ct);

    public Task ResizeAsync(int cols, int rows, CancellationToken ct)
        => _inner.ResizeAsync(cols, rows, ct);

    public ValueTask DisposeAsync() => _inner.DisposeAsync();

    internal static Domain.Contracts.Interfaces.IExecSession Map(Hosting.DockerClient.Services.IExecSession session)
        => new LocalExecSessionAdapter(session);
}
