using System.Text.Json;
using System.Text.Json.Serialization;
using Domain;
using Domain.Entities;
using Domain.Entities.Platforms;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class PlatformConfiguration : IEntityTypeConfiguration<Platform>
{
    public void Configure(EntityTypeBuilder<Platform> builder)
    {
        builder.ToTable("Platforms");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.Address).HasDatabaseName("AddressIndex").IsUnique();

        // Props
        builder.Property(p => p.Name).HasMaxLength(128);
        builder.Property(p => p.Address).HasMaxLength(128);
        builder.Property(p => p.PlatformDescriptor).IsRequired();
       
        builder.Property(p => p.ServerVersion).HasMaxLength(32);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);
        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.PlatformId).OnDelete(DeleteBehavior.Cascade);

        // Converters
        builder.Property(p => p.Status).HasConversion(
            v => v.ToString(),
            v => Enum.Parse<PlatformStatus>(v));
        builder.Property(p => p.Type).HasConversion(
            v => v.ToString(),
            v => Enum.Parse<PlatformType>(v));

        builder.Property(p => p.PlatformDescriptor).HasColumnType("TEXT")
           .HasConversion(
               v => JsonSerializer.Serialize(v, PlatformJsonContext.Default.PlatformDescriptor),
               v => JsonSerializer.Deserialize(v, PlatformJsonContext.Default.PlatformDescriptor)!);

        // Backing field
        builder.Metadata.FindNavigation(nameof(Platform.Stats))?.SetPropertyAccessMode(PropertyAccessMode.Field);
    }
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default, PropertyNameCaseInsensitive = true)]
[JsonSerializable(typeof(Platform))]
[JsonSerializable(typeof(PlatformStat))]
[JsonSerializable(typeof(ICollection<PlatformStat>))]
[JsonSerializable(typeof(DockerPlatformDescriptor))]
[JsonSerializable(typeof(DockerSwarmPlatformDescriptor))]
[JsonSerializable(typeof(KubernetesPlatformDescriptor))]
[JsonSerializable(typeof(ICollection<SwarmPeer>))]
[JsonSerializable(typeof(PlatformDescriptor))]
public partial class PlatformJsonContext : JsonSerializerContext
{
}