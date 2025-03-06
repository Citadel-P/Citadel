using Infrastructure.Entities;
using Infrastructure;
using Application.Features.Registries.Commands;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record CreateRegistryInput(string Name, string Url, RegistryDiscriminator Discriminator, IRegistryConfiguration Configuration)
{
    internal CreateRegistry ToCommand() => new(Name, Url, Discriminator, Configuration);
};