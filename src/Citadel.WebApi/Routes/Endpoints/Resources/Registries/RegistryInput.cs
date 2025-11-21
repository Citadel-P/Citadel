using System.Text.Json.Serialization;
using Application.Features.Registries.Commands;
using Domain;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryInput(
    string Name, 
    string Url,
    [property: JsonConverter(typeof(Citadel.GeneratedConverters.SafeRegistryTypeConverter))]
    RegistryType Type,
    RegistryConfigurationBase Configuration
    )
{
    internal CreateRegistry ToCommand() => new(Name, Url, Type, Configuration);
}