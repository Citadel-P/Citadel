using System.Text.Json;
using System.Text.Json.Serialization;
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
        builder.Property(p => p.Ports).HasColumnType("TEXT")
            .HasConversion(
                v => JsonSerializer.Serialize(v, ContainerPortsContext.Default.ICollectionContainerPort),
                v => JsonSerializer.Deserialize(v, ContainerPortsContext.Default.ICollectionContainerPort) ?? Array.Empty<ContainerPort>());

        // Converters
        builder.Property(p => p.State).HasConversion(
                                                v => v.ToString(),
                                                v => Enum.Parse<ContainerStateStatus>(v));

        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.ContainerInfoId).OnDelete(DeleteBehavior.Cascade);
    }
}


[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(ICollection<ContainerPort>))]
public partial class ContainerPortsContext : JsonSerializerContext
{
}