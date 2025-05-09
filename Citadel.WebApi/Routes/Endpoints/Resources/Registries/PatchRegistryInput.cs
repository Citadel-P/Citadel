using Infrastructure.Entities;
using Infrastructure;
using Application.Features.Registries.Commands;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record PatchRegistryInput(Guid Id, string Name, string Url, RegistryDiscriminator Discriminator, RegistryConfigurationBase Configuration)
{
    internal PatchRegistry ToCommand() => new(Id, Discriminator, Name, Url, Configuration);
};