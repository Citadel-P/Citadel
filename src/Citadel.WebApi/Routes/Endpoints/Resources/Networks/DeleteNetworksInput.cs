using Application.Features.Networks.Commands;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record DeleteNetworksInput(Guid PlatformId, string[] Ids)
{
   internal DeleteNetworks ToCommand() => new (PlatformId, Ids);
};
