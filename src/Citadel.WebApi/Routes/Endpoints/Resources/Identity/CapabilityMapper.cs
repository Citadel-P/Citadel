using Hosting.Common;
using Hosting.Common.Attributes;

namespace WebApi.Routes.Endpoints.Resources.Identity;

public record ResourceCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute
);

public sealed record PlatformCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanViewLogs,
    bool CanInspect,
    bool CanOpenTerminal,
    bool CanPull
) : ResourceCapabilities(
    CanRead,
    CanWrite,
    CanExecute)
{
    public static PlatformCapabilities Empty => new(false, false, false, false, false, false, false);
}

public sealed record SwarmCapabilities(
    bool CanViewCluster,
    bool CanManageNodes,
    bool CanViewServices,
    bool CanManageNetworks,
    bool CanManageSecrets,
    bool CanManageConfigs,
    bool SupportsDeploymentApply,
    bool SupportsStackApply,
    bool CanViewManagerLocalResources,
    bool SupportsServiceStatus,
    bool SupportsClusterVolumes);

public sealed record ImageCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanInspect,
    bool CanPull
) : ResourceCapabilities(
    CanRead,
    CanWrite,
    CanExecute)
{
    public static ImageCapabilities Empty => new(false, false, false, false, false);
}

public sealed record VolumeCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanInspect,
    bool CanBrowse,
    bool CanDownload
) : ResourceCapabilities(
    CanRead,
    CanWrite,
    CanExecute);

public sealed record NetworkCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanInspect
) : ResourceCapabilities(
    CanRead,
    CanWrite,
    CanExecute);

public sealed record DeploymentCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanViewLogs,
    bool CanInspect,
    bool CanOpenTerminal,
    bool CanPull,
    bool CanApply,
    bool CanViewResourceBindings
) : ResourceCapabilities(
    CanRead,
    CanWrite,
    CanExecute);

public sealed record SwarmServiceCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanViewLogs,
    bool CanInspect,
    bool CanApply,
    bool CanViewResourceBindings
) : ResourceCapabilities(CanRead, CanWrite, CanExecute);

public sealed record StackCapabilities(
    bool CanRead,
    bool CanWrite,
    bool CanExecute,
    bool CanViewLogs,
    bool CanInspect,
    bool CanOpenTerminal,
    bool CanPull,
    bool CanApply,
    bool CanViewResourceBindings,
    bool CanViewReleases
) : ResourceCapabilities(
    CanRead,
    CanWrite,
    CanExecute);

public static class CapabilityMapper
{
    public static PlatformCapabilities ToPlatformCapabilities(PermissionMetadata permission)
    {
        var common = ToResourceCapabilities(permission);

        return new PlatformCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,

            CanViewLogs:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Logs) != 0,

            CanInspect:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Inspect) != 0,

            CanOpenTerminal:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Terminal) != 0,

            CanPull:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Pull) != 0
        );
    }

    public static ImageCapabilities ToImageCapabilities(PermissionMetadata permission)
    {
        var common = ToResourceCapabilities(permission);

        return new ImageCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,

            CanInspect:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Inspect) != 0,

            CanPull:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Pull) != 0
        );
    }

    public static VolumeCapabilities ToVolumeCapabilities(
        PermissionMetadata permission,
        PermissionMetadata? contentPermission = null)
    {
        var common = ToResourceCapabilities(permission);
        var content = contentPermission ?? permission;

        return new VolumeCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,

            CanInspect:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Inspect) != 0,

            CanBrowse:
                content.Has(PermissionLevel.Read, SpecificPermission.Browse),

            CanDownload:
                content.Has(PermissionLevel.Read, SpecificPermission.Download)
        );
    }

    public static SwarmCapabilities? ToSwarmCapabilities(
        Domain.Entities.Platforms.Platform platform,
        PermissionMetadata permission)
    {
        if (platform.PlatformDescriptor is not Domain.Entities.Platforms.DockerSwarmPlatformDescriptor)
            return null;

        var common = ToResourceCapabilities(permission);
        return new SwarmCapabilities(
            CanViewCluster: common.CanRead,
            CanManageNodes: common.CanWrite && platform.Status == Domain.PlatformStatus.Online,
            CanViewServices: common.CanRead,
            CanManageNetworks: common.CanWrite && platform.Status == Domain.PlatformStatus.Online,
            CanManageSecrets: common.CanWrite && platform.Status == Domain.PlatformStatus.Online,
            CanManageConfigs: common.CanWrite && platform.Status == Domain.PlatformStatus.Online,
            SupportsDeploymentApply: false,
            SupportsStackApply: false,
            CanViewManagerLocalResources: common.CanRead,
            SupportsServiceStatus: false,
            SupportsClusterVolumes: false);
    }

    public static NetworkCapabilities ToNetworkCapabilities(PermissionMetadata permission)
    {
        var common = ToResourceCapabilities(permission);

        return new NetworkCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,

            CanInspect:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Inspect) != 0
        );
    }

    public static DeploymentCapabilities ToDeploymentCapabilities(
        PermissionMetadata permission)
    {
        var common = ToResourceCapabilities(permission);

        return new DeploymentCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,

            CanViewLogs:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Logs) != 0,

            CanInspect:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Inspect) != 0,

            CanOpenTerminal:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Terminal) != 0,

            CanPull:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Pull) != 0,

            CanApply:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Apply) != 0,

            CanViewResourceBindings:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.ResourceBindings) != 0
        );
    }

    public static SwarmServiceCapabilities ToSwarmServiceCapabilities(PermissionMetadata permission)
    {
        var common = ToResourceCapabilities(permission);
        return new SwarmServiceCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,
            CanViewLogs: common.CanRead && (permission.SpecificPermissions & SpecificPermission.Logs) != 0,
            CanInspect: common.CanRead && (permission.SpecificPermissions & SpecificPermission.Inspect) != 0,
            CanApply: common.CanRead && (permission.SpecificPermissions & SpecificPermission.Apply) != 0,
            CanViewResourceBindings: common.CanRead && (permission.SpecificPermissions & SpecificPermission.ResourceBindings) != 0);
    }

    public static StackCapabilities ToStackCapabilities(
        PermissionMetadata permission)
    {
        var common = ToResourceCapabilities(permission);

        return new StackCapabilities(
            common.CanRead,
            common.CanWrite,
            common.CanExecute,

            CanViewLogs:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Logs) != 0,

            CanInspect:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Inspect) != 0,

            CanOpenTerminal:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Terminal) != 0,

            CanPull:
               common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Pull) != 0,

            CanApply:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Apply) != 0,

            CanViewResourceBindings:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.ResourceBindings) != 0,

            CanViewReleases:
                common.CanRead &&
                (permission.SpecificPermissions & SpecificPermission.Releases) != 0
        );
    }

    public static ResourceCapabilities ToResourceCapabilities(
        PermissionMetadata permission)
    {
        var level = permission.PermissionLevel;
        var canExecute =
            (level & PermissionLevel.Execute) != 0;

        var canWrite =
            canExecute ||
            (level & PermissionLevel.Write) != 0;

        var canRead =
            canWrite ||
            (level & PermissionLevel.Read) != 0;

        return new ResourceCapabilities(
            CanRead: canRead,
            CanWrite: canWrite,
            CanExecute: canExecute
        );
    }
}
