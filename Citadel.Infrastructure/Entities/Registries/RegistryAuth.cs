using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Infrastructure.Entities;

internal sealed record RegistryAuth(string Username, string Password, string ServerAddress)
{
    internal string GetAuth()
        => Convert.ToBase64String(Encoding.UTF8.GetBytes(JsonSerializer.Serialize(new RegistryAuth(Username, Password, ServerAddress), typeof(RegistryAuth), RegistryAuthContext.Default)));
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(RegistryAuth))]
internal partial class RegistryAuthContext : JsonSerializerContext
{
}

