using System.Text.Json;
using System.Text.Json.Serialization;
using Infrastructure.Entities;
using Infrastructure.Entities.Registries;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class RegistryConfiguration : IEntityTypeConfiguration<Registry>
{
    public void Configure(EntityTypeBuilder<Registry> builder)
    {
        builder.ToTable("Registries");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.Name).IsUnique();

        // Properties
        builder.Property(p => p.Configuration).IsRequired();
        builder.Property(p => p.Name).HasMaxLength(128).IsRequired();
        builder.Property(p => p.Url).HasMaxLength(256).IsRequired();

        // Converters
        builder.Property(p => p.Type).HasConversion(
                                                v => v.ToString(),
                                                v => Enum.Parse<RegistryType>(v));

        builder.Property(p => p.Configuration).HasColumnType("TEXT")
            .HasConversion(
                v => JsonSerializer.Serialize(v, RegistryJsonContext.Default.RegistryConfigurationBase),
                v => JsonSerializer.Deserialize(v, RegistryJsonContext.Default.RegistryConfigurationBase)!);

    }
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(Registry))]
[JsonSerializable(typeof(AWSRegistry))]
[JsonSerializable(typeof(AzureRegistry))]
[JsonSerializable(typeof(GitlabRegistry))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(GitHubRegistry))]
[JsonSerializable(typeof(RegistryConfigurationBase))]
public partial class RegistryJsonContext : JsonSerializerContext
{
}