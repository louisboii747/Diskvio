use diskvio_core::list_disks;

fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if matches!(
        arguments.first().map(String::as_str),
        Some("--partition-layout" | "--partition-plan")
    ) {
        let request = match arguments.as_slice() {
            [flag,identifier] if flag == "--partition-layout" => Ok(diskvio_core::PartitionPlanningRequest::Discover { identifier:identifier.clone() }),
            [flag,path] if flag == "--partition-plan" => std::fs::read(path).map_err(|e|e.to_string())
                .and_then(|bytes|serde_json::from_slice(&bytes).map_err(|e|e.to_string())),
            _ => Err("Usage: diskvio-cli --partition-layout <physical-identifier> | --partition-plan <request.json>".into()),
        };
        match request {
            Ok(request) => {
                let response = diskvio_core::preview_partition_operation(&request);
                match serde_json::to_string_pretty(&response) {
                    Ok(json) => println!("{json}"),
                    Err(error) => {
                        eprintln!("Error: {error}");
                        std::process::exit(1);
                    }
                }
                if !response.validation.valid {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--json") {
        match diskvio_core::disk_inventory() {
            Ok(inventory) => match serde_json::to_string_pretty(&inventory) {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("Error: {error}");
                    std::process::exit(1);
                }
            },
            Err(error) => {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
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
