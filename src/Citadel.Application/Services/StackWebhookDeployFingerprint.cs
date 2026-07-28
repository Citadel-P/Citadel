using Domain;
using Domain.Entities.Stacks;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace Application.Services;

internal static class StackWebhookDeployFingerprint
{
    internal static string Compute(GitStack spec)
    {
        var normalized = spec with
        {
            BuildImageBindings = spec.BuildImageBindings?
                .Select(static binding => binding.ClearProvenance())
                .ToArray()
        };
        var json = JsonSerializer.Serialize<StackSpec>(
            normalized,
            StackJsonContext.Default.StackSpec);
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(json)));
    }
}
