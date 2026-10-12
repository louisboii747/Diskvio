use diskvio_core::*;

const MIB: u64 = PARTITION_ALIGNMENT_BYTES;

fn identity() -> PhysicalDiskIdentity {
    PhysicalDiskIdentity {
        unique_id: "naa.test-disk-001".into(),
        unique_id_format: 8,
        serial_number: "TEST001".into(),
    }
}

fn inventory() -> PartitionPlanningInventory {
    PartitionPlanningInventory {
        disks: vec![PlanningDisk {
            identifier: "PhysicalDrive2".into(),
            name: "Synthetic test disk".into(),
            identity: Some(identity()),
            device_path: Some("test-interface-001".into()),
            partition_table: PartitionScheme::Gpt,
            geometry: Some(GptGeometry {
                disk_guid: "aaaaaaaa-1111-2222-3333-444444444444".into(),
                disk_size_bytes: 100 * MIB,
                usable_start_bytes: 34 * 512,
                usable_end_bytes: 100 * MIB - 33 * 512,
                logical_sector_bytes: 512,
                physical_sector_bytes: 4096,
                sector_alignment_offset_bytes: 0,
                max_partition_count: 128,
            }),
            safety: PartitionDiskSafety {
                external: Some(true),
                system: Some(false),
                boot: Some(false),
                boot_from_disk: Some(false),
                read_only: Some(false),
                offline: Some(false),
                clustered: Some(false),
                storage_pool: Some(false),
                dynamic: Some(false),
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn partition(number: u32, offset: u64, size: u64) -> PlanningPartition {
    PlanningPartition {
        partition_id: Some(format!("{number:08x}-1111-2222-3333-444444444444")),
        number: Some(number),
        offset_bytes: Some(offset),
        size_bytes: Some(size),
        gpt_type: Some(BASIC_DATA_GPT_TYPE.into()),
        attributes: Some(0),
        service_partition: Some(false),
        safety: PartitionSafety {
            flags: DeviceSafety {
                system: Some(false),
                boot: Some(false),
                recovery: Some(false),
                hidden: Some(false),
                read_only: Some(false),
                offline: Some(false),
                page_file: Some(false),
            },
            active: Some(false),
            shadow_copy: Some(false),
            encrypted: Some(false),
            hosts_windows: Some(false),
            volumes_complete: Some(true),
        },
        volumes: vec![PartitionVolumeReference {
            identifier: format!(r"\\?\Volume{{{number:08x}-aaaa-bbbb-cccc-111111111111}}\"),
            mount_points: vec!["E:\\".into()],
        }],
        ..Default::default()
    }
}

fn discover(i: &PartitionPlanningInventory) -> PartitionPlanningResponse {
    plan_partition_in(
        i,
        &PartitionPlanningRequest::Discover {
            identifier: "PhysicalDrive2".into(),
        },
    )
}

fn create(i: &PartitionPlanningInventory) -> PartitionPlanningRequest {
    let discovery = discover(i).discovery.unwrap();
    PartitionPlanningRequest::Create {
        identifier: discovery.identifier,
        expected_identity: identity(),
        expected_layout: discovery.layout_snapshot.unwrap(),
        test_disk: TestDiskDesignation {
            identity: identity(),
            purpose: TestDiskPurpose::ExpendableExternalTestDisk,
        },
        region: discovery.regions[0].identity.clone(),
        requested_size_bytes: 4 * MIB,
        requested_start_offset_bytes: None,
        required_alignment_bytes: MIB,
        partition_table: PartitionScheme::Gpt,
        proposed_gpt_type: BASIC_DATA_GPT_TYPE.into(),
    }
}

fn delete(i: &PartitionPlanningInventory) -> PartitionPlanningRequest {
    let discovery = discover(i).discovery.unwrap();
    let p = &i.disks[0].partitions[0];
    PartitionPlanningRequest::Delete {
        identifier: discovery.identifier,
        expected_identity: identity(),
        expected_layout: discovery.layout_snapshot.unwrap(),
        test_disk: TestDiskDesignation {
            identity: identity(),
            purpose: TestDiskPurpose::ExpendableExternalTestDisk,
        },
        partition: PartitionTarget {
            partition_id: p.partition_id.clone().unwrap(),
            number: p.number.unwrap(),
            offset_bytes: p.offset_bytes.unwrap(),
            size_bytes: p.size_bytes.unwrap(),
            gpt_type: p.gpt_type.clone().unwrap(),
        },
    }
}

fn rejects(response: PartitionPlanningResponse, code: PlanningErrorCode) {
    assert!(!response.validation.valid);
    assert!(
        response.validation.issues.iter().any(|i| i.code == code),
        "{:?}",
        response.validation.issues
    );
    assert!(response.create_preview.is_none());
    assert!(!response.execution_supported);
    assert!(response.preview_only);
}

#[test]
fn empty_gpt_uses_authoritative_bounds_and_excludes_metadata() {
    let response = discover(&inventory());
    assert!(response.validation.valid);
    let discovery = response.discovery.unwrap();
    let r = &discovery.regions[0];
    assert_eq!(r.identity.start_offset_bytes, 34 * 512);
    assert_eq!(r.identity.end_offset_bytes, 100 * MIB - 33 * 512);
    assert_eq!(r.first_usable_aligned_offset_bytes, Some(MIB));
    assert_eq!(r.last_usable_aligned_boundary_bytes, Some(99 * MIB));
    assert_eq!(r.maximum_allocatable_bytes, 98 * MIB);
    assert!(r.validation.valid);
}

#[test]
fn multiple_partitions_produce_separate_regions_independent_of_order() {
    let mut i = inventory();
    i.disks[0].partitions = vec![
        partition(2, 20 * MIB, 10 * MIB),
        partition(1, 4 * MIB, 6 * MIB),
    ];
    let discovery = discover(&i).discovery.unwrap();
    assert_eq!(discovery.regions.len(), 3);
    assert_eq!(
        discovery
            .regions
            .iter()
            .map(|r| r.maximum_allocatable_bytes)
            .collect::<Vec<_>>(),
        [3 * MIB, 10 * MIB, 69 * MIB]
    );
    i.disks[0].partitions.reverse();
    assert_eq!(
        discover(&i).discovery.unwrap().layout_snapshot,
        discovery.layout_snapshot
    );
}

#[test]
fn tiny_gap_is_reported_but_never_allocatable() {
    let mut i = inventory();
    i.disks[0].partitions = vec![
        partition(1, MIB, 9 * MIB),
        partition(2, 10 * MIB + 512, 4 * MIB),
    ];
    let d = discover(&i).discovery.unwrap();
    let gap = &d.regions[1];
    assert_eq!(gap.capacity_bytes, 512);
    assert_eq!(gap.maximum_allocatable_bytes, 0);
    assert!(!gap.validation.valid);
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create { region, .. } = &mut request {
        *region = gap.identity.clone();
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::InsufficientAllocatableSpace,
    );
}

#[test]
fn creation_preview_is_exact_and_does_not_change_inventory() {
    let i = inventory();
    let before = serde_json::to_value(&i.disks).unwrap();
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create {
        requested_start_offset_bytes,
        ..
    } = &mut request
    {
        *requested_start_offset_bytes = Some(3 * MIB);
    }
    let response = plan_partition_in(&i, &request);
    assert!(response.validation.valid);
    assert!(!response.execution_supported);
    let p = response.create_preview.unwrap();
    assert_eq!(
        (
            p.start_offset_bytes,
            p.end_offset_bytes,
            p.allocated_size_bytes
        ),
        (3 * MIB, 7 * MIB, 4 * MIB)
    );
    assert_eq!(p.remaining_allocatable_capacity_bytes, 94 * MIB);
    assert_eq!(
        p.remaining_region_capacity_bytes,
        100 * MIB - 67 * 512 - 4 * MIB
    );
    assert_eq!(serde_json::to_value(&i.disks).unwrap(), before);
}

#[test]
fn size_misalignment_zero_and_below_minimum_are_rejected() {
    let i = inventory();
    for size in [0, 1, MIB - 1, MIB + 512, u64::MAX] {
        let mut request = create(&i);
        if let PartitionPlanningRequest::Create {
            requested_size_bytes,
            ..
        } = &mut request
        {
            *requested_size_bytes = size;
        }
        rejects(
            plan_partition_in(&i, &request),
            PlanningErrorCode::InvalidParameters,
        );
    }
}

#[test]
fn starts_alignment_insufficient_capacity_and_overflow_are_rejected() {
    let i = inventory();
    for start in [0, MIB + 512, 100 * MIB, u64::MAX - MIB + 1] {
        let mut request = create(&i);
        if let PartitionPlanningRequest::Create {
            requested_start_offset_bytes,
            ..
        } = &mut request
        {
            *requested_start_offset_bytes = Some(start);
        }
        rejects(
            plan_partition_in(&i, &request),
            PlanningErrorCode::InvalidParameters,
        );
    }
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create {
        requested_size_bytes,
        ..
    } = &mut request
    {
        *requested_size_bytes = 99 * MIB;
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::InsufficientAllocatableSpace,
    );
}

#[test]
fn maximum_allocation_and_minimum_allocation_are_supported() {
    let i = inventory();
    for size in [MIB, 98 * MIB] {
        let mut request = create(&i);
        if let PartitionPlanningRequest::Create {
            requested_size_bytes,
            ..
        } = &mut request
        {
            *requested_size_bytes = size;
        }
        let r = plan_partition_in(&i, &request);
        assert!(r.validation.valid);
        assert_eq!(r.create_preview.unwrap().allocated_size_bytes, size);
    }
}

#[test]
fn caller_cannot_override_partition_type_scheme_or_alignment() {
    let i = inventory();
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create {
        proposed_gpt_type, ..
    } = &mut request
    {
        *proposed_gpt_type = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b".into();
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::UnsupportedOperation,
    );
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create {
        partition_table, ..
    } = &mut request
    {
        *partition_table = PartitionScheme::Mbr;
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::UnsupportedOperation,
    );
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create {
        required_alignment_bytes,
        ..
    } = &mut request
    {
        *required_alignment_bytes = 512;
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::InvalidParameters,
    );
}

#[test]
fn overlap_out_of_bounds_zero_and_overflow_extents_have_no_regions() {
    for parts in [
        vec![partition(1, MIB, 10 * MIB), partition(2, 5 * MIB, 10 * MIB)],
        vec![partition(1, 0, MIB)],
        vec![partition(1, 99 * MIB, 2 * MIB)],
        vec![partition(1, MIB, 0)],
        vec![partition(1, u64::MAX - 511, 1024)],
        vec![partition(1, MIB + 1, MIB)],
    ] {
        let mut i = inventory();
        i.disks[0].partitions = parts;
        let r = discover(&i);
        assert!(r.discovery.as_ref().unwrap().regions.is_empty());
        rejects(r, PlanningErrorCode::InvalidGeometry);
    }
}

#[test]
fn gpt_invalid_boundaries_metadata_reservations_and_underflow_are_rejected() {
    for (start, end, size) in [
        (0, 99 * MIB, 100 * MIB),
        (34 * 512, 100 * MIB, 100 * MIB),
        (99 * MIB, MIB, 100 * MIB),
        (34 * 512, 34 * 512, 100 * MIB),
        (34 * 512, 99 * MIB, 512),
        (u64::MAX, 1, u64::MAX),
    ] {
        let mut i = inventory();
        let g = i.disks[0].geometry.as_mut().unwrap();
        g.usable_start_bytes = start;
        g.usable_end_bytes = end;
        g.disk_size_bytes = size;
        rejects(discover(&i), PlanningErrorCode::InvalidGeometry);
    }
}

#[test]
fn missing_geometry_or_partition_fields_are_not_invented() {
    let mut i = inventory();
    i.disks[0].geometry = None;
    rejects(discover(&i), PlanningErrorCode::MissingMetadata);
    for field in [
        "partition_id",
        "number",
        "offset_bytes",
        "size_bytes",
        "gpt_type",
        "attributes",
        "service_partition",
    ] {
        let mut i = inventory();
        let mut p = serde_json::to_value(partition(1, MIB, 4 * MIB)).unwrap();
        p[field] = serde_json::Value::Null;
        i.disks[0].partitions = vec![serde_json::from_value(p).unwrap()];
        rejects(discover(&i), PlanningErrorCode::MissingMetadata);
    }
}

#[test]
fn mbr_raw_unknown_and_apple_layouts_are_explicitly_unsupported() {
    for scheme in [
        PartitionScheme::Mbr,
        PartitionScheme::None,
        PartitionScheme::Unknown,
        PartitionScheme::ApplePartitionMap,
    ] {
        let mut i = inventory();
        i.disks[0].partition_table = scheme;
        let r = discover(&i);
        assert!(r.discovery.as_ref().unwrap().regions.is_empty());
        rejects(r, PlanningErrorCode::UnsupportedOperation);
    }
}

#[test]
fn sector_geometry_is_conservative_and_supports_4k_native() {
    let mut i = inventory();
    let g = i.disks[0].geometry.as_mut().unwrap();
    g.logical_sector_bytes = 4096;
    g.physical_sector_bytes = 4096;
    g.usable_start_bytes = 6 * 4096;
    g.usable_end_bytes = 100 * MIB - 5 * 4096;
    assert!(discover(&i).validation.valid);
    for (logical, physical, offset) in [
        (0, 4096, 0),
        (512, 0, 0),
        (512, 1000, 0),
        (4096, 512, 0),
        (512, 4096, 512),
        (8192, 8192, 0),
    ] {
        let mut i = inventory();
        let g = i.disks[0].geometry.as_mut().unwrap();
        g.logical_sector_bytes = logical;
        g.physical_sector_bytes = physical;
        g.sector_alignment_offset_bytes = offset;
        rejects(discover(&i), PlanningErrorCode::UnsupportedOperation);
    }
}

#[test]
fn full_gpt_entry_table_blocks_creation_but_allows_deletion_preview() {
    let mut i = inventory();
    i.disks[0].partitions = vec![partition(1, MIB, MIB)];
    i.disks[0].geometry.as_mut().unwrap().max_partition_count = 1;
    rejects(
        plan_partition_in(&i, &create(&i)),
        PlanningErrorCode::UnsupportedOperation,
    );
    assert!(plan_partition_in(&i, &delete(&i)).validation.valid);
}

#[test]
fn deletion_preview_carries_volumes_protection_and_data_loss_warning() {
    let mut i = inventory();
    i.disks[0].partitions = vec![partition(1, MIB, 10 * MIB)];
    let r = plan_partition_in(&i, &delete(&i));
    assert!(r.validation.valid);
    let p = r.delete_preview.unwrap();
    assert!(p.eligible);
    assert_eq!(p.parent_disk, "PhysicalDrive2");
    assert_eq!(p.partition.size_bytes, Some(10 * MIB));
    assert_eq!(p.partition.volumes.len(), 1);
    assert!(p.data_loss_warning.contains("permanent data loss"));
}

#[test]
fn protected_and_unsupported_types_protect_the_whole_disk() {
    for (kind, code) in [
        (
            "c12a7328-f81f-11d2-ba4b-00a0c93ec93b",
            PlanningErrorCode::ProtectedTarget,
        ),
        (
            "de94bba4-06d1-4d40-a16a-bfd50179d6ac",
            PlanningErrorCode::ProtectedTarget,
        ),
        (
            "e3c9e316-0b5c-4db8-817d-f92df00215ae",
            PlanningErrorCode::ProtectedTarget,
        ),
        (
            "af9b60a0-1431-4f62-bc68-3311714a69ad",
            PlanningErrorCode::UnsupportedOperation,
        ),
        (
            "11111111-1111-1111-1111-111111111111",
            PlanningErrorCode::UnsupportedOperation,
        ),
    ] {
        let mut i = inventory();
        i.disks[0].partitions = vec![partition(1, MIB, MIB), partition(2, 5 * MIB, MIB)];
        i.disks[0].partitions[1].gpt_type = Some(kind.into());
        let r = plan_partition_in(&i, &delete(&i));
        assert!(!r.delete_preview.as_ref().unwrap().eligible);
        rejects(r, code);
    }
}

#[test]
fn every_disk_safety_unknown_or_protected_flag_blocks_planning() {
    for field in [
        "system",
        "boot",
        "boot_from_disk",
        "read_only",
        "offline",
        "clustered",
        "storage_pool",
        "dynamic",
    ] {
        for value in [serde_json::Value::Bool(true), serde_json::Value::Null] {
            let mut i = inventory();
            let mut safety = serde_json::to_value(&i.disks[0].safety).unwrap();
            safety[field] = value.clone();
            i.disks[0].safety = serde_json::from_value(safety).unwrap();
            rejects(
                discover(&i),
                if value.is_null() {
                    PlanningErrorCode::MissingMetadata
                } else {
                    PlanningErrorCode::ProtectedTarget
                },
            );
        }
    }
    for flag in [None, Some(false)] {
        let mut i = inventory();
        i.disks[0].safety.external = flag;
        rejects(
            discover(&i),
            if flag.is_none() {
                PlanningErrorCode::MissingMetadata
            } else {
                PlanningErrorCode::ProtectedTarget
            },
        );
    }
}

#[test]
fn every_partition_safety_unknown_or_protected_flag_blocks_planning() {
    for field in [
        "system",
        "boot",
        "recovery",
        "hidden",
        "read_only",
        "offline",
        "page_file",
    ] {
        for value in [serde_json::Value::Bool(true), serde_json::Value::Null] {
            let mut i = inventory();
            let mut p = partition(1, MIB, MIB);
            let mut flags = serde_json::to_value(&p.safety.flags).unwrap();
            flags[field] = value.clone();
            p.safety.flags = serde_json::from_value(flags).unwrap();
            i.disks[0].partitions = vec![p];
            rejects(
                discover(&i),
                if value.is_null() {
                    PlanningErrorCode::MissingMetadata
                } else {
                    PlanningErrorCode::ProtectedTarget
                },
            );
        }
    }
    for field in ["active", "shadow_copy", "encrypted", "hosts_windows"] {
        for value in [serde_json::Value::Bool(true), serde_json::Value::Null] {
            let mut i = inventory();
            let mut p = serde_json::to_value(partition(1, MIB, MIB)).unwrap();
            p["safety"][field] = value.clone();
            i.disks[0].partitions = vec![serde_json::from_value(p).unwrap()];
            rejects(
                discover(&i),
                if value.is_null() {
                    PlanningErrorCode::MissingMetadata
                } else {
                    PlanningErrorCode::ProtectedTarget
                },
            );
        }
    }
    let mut i = inventory();
    let mut p = partition(1, MIB, MIB);
    p.safety.volumes_complete = None;
    i.disks[0].partitions = vec![p];
    rejects(discover(&i), PlanningErrorCode::MissingMetadata);
}

#[test]
fn attributes_and_service_partitions_cannot_be_force_deleted() {
    for attributes in [1, 1 << 60, 1 << 62, 1 << 63] {
        let mut i = inventory();
        let mut p = partition(1, MIB, MIB);
        p.attributes = Some(attributes);
        i.disks[0].partitions = vec![p];
        rejects(
            plan_partition_in(&i, &delete(&i)),
            PlanningErrorCode::ProtectedTarget,
        );
    }
    let mut i = inventory();
    let mut p = partition(1, MIB, MIB);
    p.service_partition = Some(true);
    i.disks[0].partitions = vec![p];
    rejects(
        plan_partition_in(&i, &delete(&i)),
        PlanningErrorCode::ProtectedTarget,
    );
}

#[test]
fn identity_replacement_and_disappearance_have_distinct_codes() {
    let mut i = inventory();
    let request = create(&i);
    i.disks[0].identity.as_mut().unwrap().serial_number = "REPLACEMENT".into();
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::StaleDeviceIdentity,
    );
    i.disks.clear();
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::DeviceDisappeared,
    );
}

#[test]
fn disk_number_reassignment_resolves_by_persistent_identity() {
    let mut i = inventory();
    let request = create(&i);
    i.disks[0].identifier = "PhysicalDrive9".into();
    let mut replacement = i.disks[0].clone();
    replacement.identifier = "PhysicalDrive2".into();
    replacement.identity = Some(PhysicalDiskIdentity {
        unique_id: "OTHER".into(),
        unique_id_format: 8,
        serial_number: "OTHER".into(),
    });
    i.disks.push(replacement);
    assert!(plan_partition_in(&i, &request).validation.valid);
}

#[test]
fn serial_vpd_and_locator_collisions_are_rejected_even_if_composite_differs() {
    for field in ["same", "serial", "unique", "locator"] {
        let mut i = inventory();
        let mut duplicate = i.disks[0].clone();
        duplicate.identifier = "PhysicalDrive3".into();
        if field == "serial" {
            duplicate.identity.as_mut().unwrap().unique_id = "OTHER".into();
        }
        if field == "unique" {
            duplicate.identity.as_mut().unwrap().serial_number = "OTHER".into();
        }
        if field == "locator" {
            duplicate.identity = Some(PhysicalDiskIdentity {
                unique_id: "OTHER".into(),
                unique_id_format: 8,
                serial_number: "OTHER".into(),
            });
            duplicate.identifier = "PhysicalDrive2".into();
        }
        i.disks.push(duplicate);
        rejects(discover(&i), PlanningErrorCode::AmbiguousIdentity);
    }
}

#[test]
fn missing_and_placeholder_identity_never_authorizes() {
    let mut i = inventory();
    i.disks[0].identity = None;
    rejects(discover(&i), PlanningErrorCode::MissingMetadata);
    for value in [
        "",
        "unknown",
        "0",
        "00000000",
        " default ",
        "1234567890",
        "Not Available",
        "Generic",
    ] {
        let mut i = inventory();
        i.disks[0].identity.as_mut().unwrap().serial_number = value.into();
        rejects(discover(&i), PlanningErrorCode::MissingMetadata);
    }
    let mut i = inventory();
    i.disks[0].identity.as_mut().unwrap().unique_id_format = 0;
    rejects(discover(&i), PlanningErrorCode::MissingMetadata);
}

#[test]
fn layout_changes_partition_count_extents_types_and_connection_invalidate_plan() {
    let i = inventory();
    let request = create(&i);
    for change in ["count", "guid", "sector", "capacity", "path", "safety"] {
        let mut changed = i.clone();
        let disk = &mut changed.disks[0];
        match change {
            "count" => disk.partitions.push(partition(1, 10 * MIB, MIB)),
            "guid" => {
                disk.geometry.as_mut().unwrap().disk_guid =
                    "bbbbbbbb-1111-2222-3333-444444444444".into()
            }
            "sector" => disk.geometry.as_mut().unwrap().physical_sector_bytes = 512,
            "capacity" => disk.geometry.as_mut().unwrap().disk_size_bytes = 101 * MIB,
            "path" => disk.device_path = Some("new-interface".into()),
            _ => disk.safety.offline = Some(true),
        }
        rejects(
            plan_partition_in(&changed, &request),
            PlanningErrorCode::ChangedPartitionLayout,
        );
    }
    let mut i = inventory();
    i.disks[0].partitions = vec![partition(1, 10 * MIB, MIB)];
    let request = delete(&i);
    for change in ["offset", "size", "type", "number", "mount"] {
        let mut changed = i.clone();
        let p = &mut changed.disks[0].partitions[0];
        match change {
            "offset" => p.offset_bytes = Some(11 * MIB),
            "size" => p.size_bytes = Some(2 * MIB),
            "type" => p.gpt_type = Some("11111111-1111-1111-1111-111111111111".into()),
            "number" => p.number = Some(2),
            _ => p.volumes[0].mount_points = vec!["F:\\".into()],
        }
        rejects(
            plan_partition_in(&changed, &request),
            PlanningErrorCode::ChangedPartitionLayout,
        );
    }
}

#[test]
fn selected_region_and_partition_metadata_must_match_exactly() {
    let mut i = inventory();
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create { region, .. } = &mut request {
        region.start_offset_bytes += 512;
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::ChangedPartitionLayout,
    );
    i.disks[0].partitions = vec![partition(1, 10 * MIB, MIB)];
    let mut request = delete(&i);
    if let PartitionPlanningRequest::Delete { partition, .. } = &mut request {
        partition.size_bytes += 512;
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::ChangedPartitionLayout,
    );
}

#[test]
fn incomplete_discovery_and_test_disk_mismatch_cannot_authorize() {
    let mut i = inventory();
    let request = create(&i);
    i.issues.push(PlanningIssue::new(
        PlanningErrorCode::MissingMetadata,
        "partial inventory",
    ));
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::MissingMetadata,
    );
    let i = inventory();
    let mut request = create(&i);
    if let PartitionPlanningRequest::Create { test_disk, .. } = &mut request {
        test_disk.identity.serial_number = "OTHER".into();
    }
    rejects(
        plan_partition_in(&i, &request),
        PlanningErrorCode::TestDiskDesignationRequired,
    );
}

#[test]
fn duplicate_partition_ids_or_numbers_are_ambiguous() {
    for field in ["id", "number"] {
        let mut i = inventory();
        let a = partition(1, 2 * MIB, MIB);
        let mut b = partition(2, 5 * MIB, MIB);
        if field == "id" {
            b.partition_id = a.partition_id.clone();
        } else {
            b.number = a.number;
        }
        i.disks[0].partitions = vec![a, b];
        rejects(discover(&i), PlanningErrorCode::AmbiguousIdentity);
    }
}

#[test]
fn typed_json_roundtrip_and_closed_request_modes() {
    let i = inventory();
    let request = create(&i);
    let json = serde_json::to_value(&request).unwrap();
    let decoded: PartitionPlanningRequest = serde_json::from_value(json.clone()).unwrap();
    assert!(plan_partition_in(&i, &decoded).validation.valid);
    let response = discover(&i);
    let value = serde_json::to_value(response).unwrap();
    let decoded: PartitionPlanningResponse = serde_json::from_value(value).unwrap();
    assert!(decoded.preview_only);
    assert!(!decoded.execution_supported);
    for field in ["action", "execute", "force", "filesystem"] {
        let mut invalid = json.clone();
        invalid[field] = true.into();
        assert!(serde_json::from_value::<PartitionPlanningRequest>(invalid).is_err());
    }
    for mode in [
        "execute",
        "format",
        "resize",
        "create_partition",
        "delete_partition",
    ] {
        let mut invalid = json.clone();
        invalid["mode"] = mode.into();
        assert!(serde_json::from_value::<PartitionPlanningRequest>(invalid).is_err());
    }
}

#[test]
fn shared_native_client_fixture_preserves_the_rust_response_contract() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/partition_planning_contract.json")).unwrap();
    let response: PartitionPlanningResponse =
        serde_json::from_value(value["planning"].clone()).unwrap();
    assert!(response.validation.valid);
    assert!(response.preview_only);
    assert!(!response.execution_supported);
    let preview = response.create_preview.as_ref().unwrap();
    assert_eq!(
        preview.end_offset_bytes - preview.start_offset_bytes,
        preview.allocated_size_bytes
    );
    assert_eq!(serde_json::to_value(response).unwrap(), value["planning"]);
}

#[test]
fn protected_regions_have_no_safely_allocatable_capacity() {
    let mut i = inventory();
    i.disks[0].safety.system = Some(true);
    let response = discover(&i);
    let region = &response.discovery.unwrap().regions[0];
    assert!(!region.validation.valid);
    assert_eq!(region.maximum_allocatable_bytes, 0);
    assert_eq!(region.aligned_capacity_bytes, 98 * MIB);
}

#[cfg(not(target_os = "windows"))]
#[test]
fn live_nonwindows_planning_is_explicitly_unsupported() {
    rejects(
        preview_partition_operation(&PartitionPlanningRequest::Discover {
            identifier: "disk2".into(),
        }),
        PlanningErrorCode::UnsupportedPlatform,
    );
}
