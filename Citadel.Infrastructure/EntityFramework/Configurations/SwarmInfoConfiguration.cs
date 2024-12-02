using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class SwarmInfoConfiguration : IEntityTypeConfiguration<SwarmInfo>
{
    public void Configure(EntityTypeBuilder<SwarmInfo> builder)
    {
        builder.ToTable("SwarmsInfo");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Properties
        builder.Property(p => p.NodeID).HasMaxLength(128);
        builder.Property(p => p.NodeAddr).HasMaxLength(128);
        builder.Property(p => p.LocalNodeState).HasMaxLength(128);
        builder.HasMany(p => p.RemoteManagers).WithOne(p => p.SwarmInfo).HasForeignKey(p => p.SwarmInfoId).OnDelete(DeleteBehavior.Cascade);
    }
}