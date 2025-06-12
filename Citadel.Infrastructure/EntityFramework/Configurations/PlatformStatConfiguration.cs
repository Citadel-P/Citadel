using Domain.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class PlatformStatConfiguration : IEntityTypeConfiguration<PlatformStat>
{
    public void Configure(EntityTypeBuilder<PlatformStat> builder)
    {
        builder.ToTable("PlatformStats");
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.Created).IsUnique();
    }
}