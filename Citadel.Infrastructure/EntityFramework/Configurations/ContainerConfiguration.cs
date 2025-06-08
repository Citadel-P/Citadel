using System.Text.Json;
using System.Text.Json.Serialization;
using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal class ContainerConfiguration : IEntityTypeConfiguration<Container>
{
    public void Configure(EntityTypeBuilder<Container> builder)
    {
        builder.ToTable("Containers");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.ContainerId).IsUnique();

        // Properties
        builder.Property(p => p.ContainerId).HasMaxLength(64);
        builder.Property(p => p.Ports).HasColumnType("TEXT")
            .HasConversion(
                v => JsonSerializer.Serialize(v, ContainerPortsContext.Default.IReadOnlyCollectionContainerPort),
                v => JsonSerializer.Deserialize(v, ContainerPortsContext.Default.IReadOnlyCollectionContainerPort) ?? Array.Empty<ContainerPort>());

        // Converters
        builder.Property(p => p.State).HasConversion(
                                                v => v.ToString(),
                                                v => Enum.Parse<ContainerStateStatus>(v));

        // Backing field
        builder.Metadata.FindNavigation(nameof(Container.Stats))?.SetPropertyAccessMode(PropertyAccessMode.Field);

        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.ContainerId).OnDelete(DeleteBehavior.Cascade);
    }
}


[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(IReadOnlyCollection<ContainerPort>))]
public partial class ContainerPortsContext : JsonSerializerContext
{
}