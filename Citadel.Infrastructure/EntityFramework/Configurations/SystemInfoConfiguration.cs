using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class SystemInfoConfiguration : IEntityTypeConfiguration<SystemInfo>
{
    public void Configure(EntityTypeBuilder<SystemInfo> builder)
    {
        builder.ToTable("SystemsInfo");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Properties
        builder.Property(p => p.OperatingSystem).HasMaxLength(128);
        builder.Property(p => p.OperatingSystem).HasMaxLength(128);
        builder.Property(p => p.Architecture).HasMaxLength(128);
        builder.Property(p => p.OsType).HasMaxLength(128);
        builder.Property(p => p.OsVersion).HasMaxLength(128);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);

        builder.HasOne(p => p.SwarmInfo).WithOne(p => p.SystemInfo).HasForeignKey<SwarmInfo>(p => p.SystemInfoId).OnDelete(DeleteBehavior.Cascade);
    }
}