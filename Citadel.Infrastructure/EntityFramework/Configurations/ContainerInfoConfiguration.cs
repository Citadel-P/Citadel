using System.Text.Json;
using System.Text.Json.Serialization;
using Google.Protobuf.Collections;
using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal class ContainerInfoConfiguration : IEntityTypeConfiguration<ContainerInfo>
{
    public void Configure(EntityTypeBuilder<ContainerInfo> builder)
    {
        builder.ToTable("ContainersInfo");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.ContainerId).IsUnique();

        // Properties
        builder.Property(p => p.ContainerId).HasMaxLength(64);
        builder.OwnsMany(p => p.Ports, cfg => cfg.ToJson());

        builder.Property(p => p.Labels).HasConversion(
              v => JsonSerializer.Serialize(v, typeof(Dictionary<string, string>), ContainerInfoConfigurationContext.Default),
              v => (Dictionary<string, string>)JsonSerializer.Deserialize(v, typeof(Dictionary<string, string>),  ContainerInfoConfigurationContext.Default ));

        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.ContainerInfoId).OnDelete(DeleteBehavior.Cascade);
    }
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(Dictionary<string, string>))]
[JsonSerializable(typeof(MapField<string, string>))]
internal partial class ContainerInfoConfigurationContext : JsonSerializerContext
{
}
