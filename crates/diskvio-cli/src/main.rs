use diskvio_core::list_disks;

fn main() {
    println!("Diskvio — Disk Inspector");
    println!("======================\n");

    match list_disks() {
        Ok(disks) => {
            println!("Found {} disk(s)\n", disks.len());

            for disk in disks {
                println!("Disk {}: {}", disk.number, disk.name);
                println!(
                    "  Capacity: {:.2} GB",
                    disk.size_bytes as f64 / 1_000_000_000.0
                );
                println!("  Interface: {}", disk.bus_type);
                println!("  Partition style: {}", disk.partition_style);
                println!();
            }
        }
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
    }
}
