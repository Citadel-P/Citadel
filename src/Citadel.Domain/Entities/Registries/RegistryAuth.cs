using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

internal sealed record RegistryAuth(string Username, string Password, string RegistryHost)
{
    internal string GetAuth()
        => Convert.ToBase64String(Encoding.UTF8.GetBytes(JsonSerializer.Serialize(new RegistryAuth(Username, Password, RegistryHost), typeof(RegistryAuth), RegistryAuthContext.Default)));
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(RegistryAuth))]
internal partial class RegistryAuthContext : JsonSerializerContext
{
}

