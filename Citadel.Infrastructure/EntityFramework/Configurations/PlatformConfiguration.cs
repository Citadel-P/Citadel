using Infrastructure.Entities;
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
        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.PlatformId).OnDelete(DeleteBehavior.Cascade);
        builder.HasOne(p => p.SystemInfo).WithOne(p => p.Platform).HasForeignKey<SystemInfo>(p => p.PlatformId).OnDelete(DeleteBehavior.Cascade);
    }
}