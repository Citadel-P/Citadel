using Infrastructure.Entities;
using Infrastructure;
using Application.Features.Registries.Commands;

namespace WebApi.Controllers.V1.Resources.Registries;

public sealed record CreateRegistryRequest(string Name, string Url, RegistryDiscriminator Discriminator, IRegistryConfiguration Configuration)
{
    internal CreateRegistry ToCommand() => new(Name, Url, Discriminator, Configuration);
};