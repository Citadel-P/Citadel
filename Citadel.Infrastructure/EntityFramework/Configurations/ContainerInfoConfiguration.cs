using System.Text.Json;
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
              v => JsonSerializer.Serialize(v, (JsonSerializerOptions)null),
              v => JsonSerializer.Deserialize<Dictionary<string, string>>(v, (JsonSerializerOptions)null));

        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.ContainerInfoId).OnDelete(DeleteBehavior.Cascade);
    }
}
