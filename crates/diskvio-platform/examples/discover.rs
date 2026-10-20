//! Opt-in read-only diagnostic. Serial numbers and stable IDs are deliberately omitted.
use diskvio_platform::{DiskPlatform, NativePlatform};
fn main() {
    match NativePlatform.discover() {
        Ok(disks) => {
            for d in disks {
                // OS paths and stable IDs can embed serials, so omit those too.
                println!(
                    "{} | {} bytes | {} | partitions: {}",
                    d.name,
                    d.capacity_bytes,
                    d.connection.as_deref().unwrap_or("unknown"),
                    d.partitions
                        .as_ref()
                        .map_or_else(|| "unavailable".into(), |ps| ps.len().to_string())
                );
                for p in d.partitions.iter().flatten() {
                    println!(
                        "  #{} offset={} length={} type={}",
                        p.number.map_or_else(|| "?".into(), |n| n.to_string()),
                        p.offset_bytes,
                        p.capacity_bytes,
                        p.partition_type.as_deref().unwrap_or("unknown")
                    );
                    for v in &p.volumes {
                        println!(
                            "    filesystem={} capacity={:?} used={:?} free={:?}",
                            v.filesystem.as_deref().unwrap_or("unknown"),
                            v.capacity_bytes,
                            v.used_bytes,
                            v.free_bytes
                        );
                    }
                }
                for limitation in d.limitations {
                    println!("  {limitation}");
                }
            }
        }
        Err(e) => {
            eprintln!("{:?}: {}", e.code, e.message);
            std::process::exit(1);
        }
    }
}
