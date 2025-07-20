using Application.Features.Registries.Commands;
using Domain;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryInput(
    string? Name, 
    string? Url,
    RegistryType? Type,
    RegistryConfigurationBase? Configuration)
{
    internal CreateRegistry ToCreateRegistryCommand() => new(Name, Url, Type.Value, Configuration);
}