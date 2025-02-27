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
        builder.Property(p => p.OperatingSystem).HasMaxLength(128);
        builder.Property(p => p.OperatingSystem).HasMaxLength(128);
        builder.Property(p => p.Architecture).HasMaxLength(128);
        builder.Property(p => p.OsType).HasMaxLength(128);
        builder.Property(p => p.OsVersion).HasMaxLength(128);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);
        builder.HasMany(p => p.Stats).WithOne().HasForeignKey(p => p.PlatformId).OnDelete(DeleteBehavior.Cascade);
        builder.HasMany(p => p.ContainersInfo).WithOne(p => p.Platform).HasForeignKey(p => p.PlatformId).IsRequired().OnDelete(DeleteBehavior.Cascade);
        builder.HasOne(p => p.SwarmInfo).WithOne(p => p.Platform).HasForeignKey<SwarmInfo>(p => p.PlatformId).OnDelete(DeleteBehavior.Cascade);
    }
}