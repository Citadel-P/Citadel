using Application.Permissions;
using Domain.Entities.Registries;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistriesView(IEnumerable<RegistryView> Registries)
{
    internal static async Task<RegistriesView> Map(IEnumerable<Registry> registries, IPermissionEvaluator permissionEvaluator)
    {
        var list = registries as Registry[] ?? [.. registries];

        if (list.Length == 0)
            return new RegistriesView([]);

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.Registry);

        var views = new RegistryView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var registry = list[i];

            var baseView = RegistryView.Map(registry);

            perms.TryGetValue(registry.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new RegistriesView(views);
    }
}
