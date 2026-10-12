use crate::{DeviceSafety, PartitionScheme};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PARTITION_ALIGNMENT_BYTES: u64 = 1_048_576;
pub const MIN_PARTITION_BYTES: u64 = PARTITION_ALIGNMENT_BYTES;
pub const BASIC_DATA_GPT_TYPE: &str = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalDiskIdentity {
    pub unique_id: String,
    pub unique_id_format: u16,
    pub serial_number: String,
}

impl PhysicalDiskIdentity {
    pub(crate) fn complete(&self) -> bool {
        matches!(self.unique_id_format, 2 | 3 | 8)
            && meaningful_identity(&self.unique_id)
            && meaningful_identity(&self.serial_number)
    }

    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.unique_id_format == other.unique_id_format
            && self.unique_id.eq_ignore_ascii_case(&other.unique_id)
            && self
                .serial_number
                .eq_ignore_ascii_case(&other.serial_number)
    }
}

fn meaningful_identity(value: &str) -> bool {
    value == value.trim()
        && !value.is_empty()
        && !value.chars().any(char::is_control)
        && ![
            "unknown",
            "none",
            "default",
            "0",
            "n/a",
            "generic",
            "not specified",
            "not available",
            "not applicable",
            "to be filled by o.e.m.",
            "123456789",
            "1234567890",
            "0123456789",
        ]
        .iter()
        .any(|placeholder| value.eq_ignore_ascii_case(placeholder))
        && value.chars().any(|c| c != '0' && c != '-' && c != ' ')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskLayoutSnapshot {
    pub version: u32,
    pub token: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GptGeometry {
    pub disk_guid: String,
    pub disk_size_bytes: u64,
    pub usable_start_bytes: u64,
    pub usable_end_bytes: u64,
    pub logical_sector_bytes: u32,
    pub physical_sector_bytes: u32,
    pub sector_alignment_offset_bytes: u32,
    pub max_partition_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartitionDiskSafety {
    pub external: Option<bool>,
    pub system: Option<bool>,
    pub boot: Option<bool>,
    pub boot_from_disk: Option<bool>,
    pub read_only: Option<bool>,
    pub offline: Option<bool>,
    pub clustered: Option<bool>,
    pub storage_pool: Option<bool>,
    pub dynamic: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartitionSafety {
    pub flags: DeviceSafety,
    pub active: Option<bool>,
    pub shadow_copy: Option<bool>,
    pub encrypted: Option<bool>,
    pub hosts_windows: Option<bool>,
    pub volumes_complete: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionVolumeReference {
    pub identifier: String,
    pub mount_points: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanningPartition {
    pub partition_id: Option<String>,
    pub number: Option<u32>,
    pub offset_bytes: Option<u64>,
    pub size_bytes: Option<u64>,
    pub gpt_type: Option<String>,
    pub attributes: Option<u64>,
    pub service_partition: Option<bool>,
    pub name: Option<String>,
    pub safety: PartitionSafety,
    pub volumes: Vec<PartitionVolumeReference>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanningDisk {
    pub identifier: String,
    pub name: String,
    pub identity: Option<PhysicalDiskIdentity>,
    pub device_path: Option<String>,
    pub partition_table: PartitionScheme,
    pub geometry: Option<GptGeometry>,
    pub safety: PartitionDiskSafety,
    pub partitions: Vec<PlanningPartition>,
    pub discovery_issues: Vec<PlanningIssue>,
}

#[derive(Debug, Clone, Default)]
pub struct PartitionPlanningInventory {
    pub disks: Vec<PlanningDisk>,
    pub issues: Vec<PlanningIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningErrorCode {
    UnsupportedOperation,
    UnsupportedPlatform,
    InvalidParameters,
    ProtectedTarget,
    StaleDeviceIdentity,
    ChangedPartitionLayout,
    MissingMetadata,
    InsufficientAllocatableSpace,
    InvalidGeometry,
    AmbiguousIdentity,
    DeviceDisappeared,
    PermissionDenied,
    DiscoveryFailed,
    TestDiskDesignationRequired,
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningIssue {
    pub code: PlanningErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_code: Option<i64>,
}

impl PlanningIssue {
    pub fn new(code: PlanningErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            platform_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningValidation {
    pub valid: bool,
    pub issues: Vec<PlanningIssue>,
}

impl PlanningValidation {
    fn from_issues(issues: Vec<PlanningIssue>) -> Self {
        Self {
            valid: issues.is_empty(),
            issues,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocatableRegionIdentity {
    pub start_offset_bytes: u64,
    pub end_offset_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocatableRegion {
    pub parent_identity: PhysicalDiskIdentity,
    pub identity: AllocatableRegionIdentity,
    pub capacity_bytes: u64,
    pub required_alignment_bytes: u64,
    pub first_usable_aligned_offset_bytes: Option<u64>,
    pub last_usable_aligned_boundary_bytes: Option<u64>,
    pub aligned_capacity_bytes: u64,
    pub maximum_allocatable_bytes: u64,
    pub partition_table: PartitionScheme,
    pub validation: PlanningValidation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionTarget {
    pub partition_id: String,
    pub number: u32,
    pub offset_bytes: u64,
    pub size_bytes: u64,
    pub gpt_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestDiskDesignation {
    pub identity: PhysicalDiskIdentity,
    pub purpose: TestDiskPurpose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestDiskPurpose {
    ExpendableExternalTestDisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum PartitionPlanningRequest {
    Discover {
        identifier: String,
    },
    Create {
        identifier: String,
        expected_identity: PhysicalDiskIdentity,
        expected_layout: DiskLayoutSnapshot,
        test_disk: TestDiskDesignation,
        region: AllocatableRegionIdentity,
        requested_size_bytes: u64,
        requested_start_offset_bytes: Option<u64>,
        required_alignment_bytes: u64,
        partition_table: PartitionScheme,
        proposed_gpt_type: String,
    },
    Delete {
        identifier: String,
        expected_identity: PhysicalDiskIdentity,
        expected_layout: DiskLayoutSnapshot,
        test_disk: TestDiskDesignation,
        partition: PartitionTarget,
    },
}

impl PartitionPlanningRequest {
    pub fn identifier(&self) -> &str {
        match self {
            Self::Discover { identifier }
            | Self::Create { identifier, .. }
            | Self::Delete { identifier, .. } => identifier,
        }
    }

    fn expected(
        &self,
    ) -> Option<(
        &PhysicalDiskIdentity,
        &DiskLayoutSnapshot,
        &TestDiskDesignation,
    )> {
        match self {
            Self::Discover { .. } => None,
            Self::Create {
                expected_identity,
                expected_layout,
                test_disk,
                ..
            }
            | Self::Delete {
                expected_identity,
                expected_layout,
                test_disk,
                ..
            } => Some((expected_identity, expected_layout, test_disk)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePartitionPreview {
    pub parent_identity: PhysicalDiskIdentity,
    pub region: AllocatableRegionIdentity,
    pub expected_layout: DiskLayoutSnapshot,
    pub partition_table: PartitionScheme,
    pub proposed_gpt_type: String,
    pub required_alignment_bytes: u64,
    pub start_offset_bytes: u64,
    pub end_offset_bytes: u64,
    pub allocated_size_bytes: u64,
    pub remaining_region_capacity_bytes: u64,
    pub remaining_allocatable_capacity_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePartitionPreview {
    pub parent_disk: String,
    pub parent_identity: Option<PhysicalDiskIdentity>,
    pub partition: PlanningPartition,
    pub eligible: bool,
    pub validation: PlanningValidation,
    pub data_loss_warning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskPlanningDiscovery {
    pub identifier: String,
    pub name: String,
    pub physical_identity: Option<PhysicalDiskIdentity>,
    pub partition_table: PartitionScheme,
    pub geometry: Option<GptGeometry>,
    pub layout_snapshot: Option<DiskLayoutSnapshot>,
    pub safety: PartitionDiskSafety,
    pub validation: PlanningValidation,
    pub regions: Vec<AllocatableRegion>,
    pub partitions: Vec<DeletePartitionPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionPlanningResponse {
    pub api_version: u32,
    pub preview_only: bool,
    pub execution_supported: bool,
    pub validation: PlanningValidation,
    pub discovery: Option<DiskPlanningDiscovery>,
    pub create_preview: Option<CreatePartitionPreview>,
    pub delete_preview: Option<DeletePartitionPreview>,
}

impl PartitionPlanningResponse {
    pub fn rejected(issue: PlanningIssue) -> Self {
        Self {
            api_version: 1,
            preview_only: true,
            execution_supported: false,
            validation: PlanningValidation::from_issues(vec![issue]),
            discovery: None,
            create_preview: None,
            delete_preview: None,
        }
    }
}

pub fn preview_partition_operation(
    request: &PartitionPlanningRequest,
) -> PartitionPlanningResponse {
    if !valid_planning_identifier(request.identifier()) {
        return PartitionPlanningResponse::rejected(PlanningIssue::new(
            PlanningErrorCode::InvalidParameters,
            "A physical disk identifier is required",
        ));
    }
    #[cfg(target_os = "windows")]
    {
        crate::platform::partition_preview(request)
    }
    #[cfg(not(target_os = "windows"))]
    {
        PartitionPlanningResponse::rejected(PlanningIssue::new(
            PlanningErrorCode::UnsupportedPlatform,
            "Partition layout allocation and create/delete previews are supported only by the Windows planning backend. Execution is unavailable on every platform.",
        ))
    }
}

pub(crate) fn valid_planning_identifier(identifier: &str) -> bool {
    (identifier
        .strip_prefix("PhysicalDrive")
        .or_else(|| identifier.strip_prefix("disk")))
    .is_some_and(|number| !number.is_empty() && number.parse::<u32>().is_ok())
        && identifier.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn issue(code: PlanningErrorCode, message: impl Into<String>) -> PlanningIssue {
    PlanningIssue::new(code, message)
}

pub(crate) fn canonical_guid(value: &str) -> String {
    value.trim_matches(['{', '}']).to_ascii_lowercase()
}

fn nonzero_guid(value: &str) -> bool {
    crate::operations::valid_uuid(value) && value != "00000000-0000-0000-0000-000000000000"
}

fn geometry_validation(disk: &PlanningDisk) -> Vec<PlanningIssue> {
    use PlanningErrorCode::*;
    let mut issues = disk.discovery_issues.clone();
    if disk.partition_table != PartitionScheme::Gpt {
        issues.push(issue(UnsupportedOperation, "Only initialized basic GPT layouts are supported. MBR, RAW, hybrid and unknown layouts cannot be allocated."));
        return issues;
    }
    let Some(g) = &disk.geometry else {
        issues.push(issue(
            MissingMetadata,
            "Authoritative GPT usable boundaries and sector geometry are required",
        ));
        return issues;
    };
    let logical = u64::from(g.logical_sector_bytes);
    let physical = u64::from(g.physical_sector_bytes);
    if ![512, 4096].contains(&logical)
        || !physical.is_power_of_two()
        || physical < logical
        || physical > PARTITION_ALIGNMENT_BYTES
        || g.sector_alignment_offset_bytes != 0
    {
        issues.push(issue(UnsupportedOperation, "Unsupported or missing sector geometry; nonzero physical-sector alignment offsets are not supported"));
        return issues;
    }
    let minimum_reserved = u64::from(g.max_partition_count)
        .checked_mul(128)
        .and_then(|bytes| bytes.checked_add(logical - 1))
        .map(|bytes| bytes / logical * logical);
    let valid_bounds = minimum_reserved
        .and_then(|entries| {
            let minimum_start = entries.checked_add(logical.checked_mul(2)?)?;
            let maximum_end = g
                .disk_size_bytes
                .checked_sub(entries.checked_add(logical)?)?;
            Some(g.usable_start_bytes >= minimum_start && g.usable_end_bytes <= maximum_end)
        })
        .unwrap_or(false);
    if !nonzero_guid(&canonical_guid(&g.disk_guid))
        || g.max_partition_count == 0
        || g.max_partition_count > 4096
        || g.disk_size_bytes > i64::MAX as u64
        || g.disk_size_bytes % logical != 0
        || g.usable_start_bytes >= g.usable_end_bytes
        || g.usable_start_bytes % logical != 0
        || g.usable_end_bytes % logical != 0
        || !valid_bounds
        || disk.partitions.len() > g.max_partition_count as usize
    {
        issues.push(issue(InvalidGeometry, "Invalid GPT usable boundaries, partition-table capacity, disk size or reserved metadata"));
    }
    let mut extents = Vec::new();
    let mut numbers = HashSet::new();
    let mut ids = HashSet::new();
    for p in &disk.partitions {
        let (Some(offset), Some(size), Some(number), Some(id), Some(kind), Some(_), Some(_)) = (
            p.offset_bytes,
            p.size_bytes,
            p.number,
            &p.partition_id,
            &p.gpt_type,
            p.attributes,
            p.service_partition,
        ) else {
            issues.push(issue(MissingMetadata, "Partition extent, number, GUID, type, attributes and service status must be reported"));
            continue;
        };
        if number == 0
            || !numbers.insert(number)
            || !ids.insert(canonical_guid(id))
            || !nonzero_guid(&canonical_guid(id))
            || !nonzero_guid(&canonical_guid(kind))
        {
            issues.push(issue(
                AmbiguousIdentity,
                "Invalid or duplicate partition number/GUID/type",
            ));
        }
        match offset.checked_add(size) {
            Some(end)
                if size > 0
                    && offset >= g.usable_start_bytes
                    && end <= g.usable_end_bytes
                    && offset % logical == 0
                    && size % logical == 0 =>
            {
                extents.push((offset, end))
            }
            _ => issues.push(issue(
                InvalidGeometry,
                "Partition extent overflows, is outside GPT usable bounds or is not sector aligned",
            )),
        }
    }
    extents.sort_unstable();
    if extents.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        issues.push(issue(
            InvalidGeometry,
            "Overlapping partition extents cannot be allocated",
        ));
    }
    issues
}

fn flag_issue(issues: &mut Vec<PlanningIssue>, flag: Option<bool>, name: &str) {
    match flag {
        Some(false) => {}
        Some(true) => issues.push(issue(
            PlanningErrorCode::ProtectedTarget,
            format!("Protected target: {name}"),
        )),
        None => issues.push(issue(
            PlanningErrorCode::MissingMetadata,
            format!("Safety status is unknown: {name}"),
        )),
    }
}

pub fn partition_safety_policy(disk: &PlanningDisk) -> PlanningValidation {
    use PlanningErrorCode::*;
    let mut issues = Vec::new();
    match disk.safety.external {
        Some(true) => {}
        Some(false) => issues.push(issue(
            ProtectedTarget,
            "Only positively identified external removable test media are eligible",
        )),
        None => issues.push(issue(
            MissingMetadata,
            "External location and removable-media evidence are required",
        )),
    }
    for (flag, name) in [
        (disk.safety.system, "system disk"),
        (disk.safety.boot, "boot disk"),
        (disk.safety.boot_from_disk, "active boot environment"),
        (disk.safety.read_only, "read-only disk"),
        (disk.safety.offline, "offline disk"),
        (disk.safety.clustered, "clustered disk"),
        (disk.safety.storage_pool, "storage pool membership"),
        (disk.safety.dynamic, "dynamic disk"),
    ] {
        flag_issue(&mut issues, flag, name);
    }
    for p in &disk.partitions {
        let f = &p.safety.flags;
        for (flag, name) in [
            (f.system, "system partition"),
            (f.boot, "boot partition"),
            (f.recovery, "recovery partition"),
            (f.hidden, "hidden partition"),
            (f.read_only, "read-only partition"),
            (f.offline, "offline partition"),
            (f.page_file, "paging-file partition"),
            (p.safety.active, "active partition"),
            (p.safety.shadow_copy, "shadow copy"),
            (
                p.safety.encrypted,
                "encrypted or BitLocker-protected partition",
            ),
            (
                p.safety.hosts_windows,
                "Windows installation or boot environment",
            ),
            (p.service_partition, "service partition"),
        ] {
            flag_issue(&mut issues, flag, name);
        }
        if p.safety.volumes_complete != Some(true) {
            issues.push(issue(
                MissingMetadata,
                "Associated-volume discovery is incomplete",
            ));
        }
        match p.gpt_type.as_deref().map(canonical_guid).as_deref() {
            Some(BASIC_DATA_GPT_TYPE) => {},
            Some("c12a7328-f81f-11d2-ba4b-00a0c93ec93b" | "de94bba4-06d1-4d40-a16a-bfd50179d6ac"
                | "e3c9e316-0b5c-4db8-817d-f92df00215ae") =>
                issues.push(issue(ProtectedTarget, "The whole disk is protected because it contains EFI, recovery or reserved partitions")),
            Some(_) => issues.push(issue(UnsupportedOperation, "The whole disk contains an unsupported partition type")),
            None => issues.push(issue(MissingMetadata, "Partition type is missing")),
        }
        if p.attributes != Some(0) {
            issues.push(issue(
                ProtectedTarget,
                "Nonzero or unknown GPT partition attributes are protected",
            ));
        }
    }
    PlanningValidation::from_issues(issues)
}

fn identity_validation(
    inventory: &PartitionPlanningInventory,
    disk: &PlanningDisk,
) -> Vec<PlanningIssue> {
    let Some(identity) = disk.identity.as_ref().filter(|id| id.complete()) else {
        return vec![issue(
            PlanningErrorCode::MissingMetadata,
            "A supported VPD identifier and nonempty hardware serial are required",
        )];
    };
    let collides = inventory
        .disks
        .iter()
        .filter(|other| {
            other.identifier == disk.identifier
                || other.identity.as_ref().is_some_and(|id| {
                    id.unique_id.eq_ignore_ascii_case(&identity.unique_id)
                        || id
                            .serial_number
                            .eq_ignore_ascii_case(&identity.serial_number)
                })
        })
        .count()
        != 1;
    if collides {
        vec![issue(
            PlanningErrorCode::AmbiguousIdentity,
            "Physical disk identifier, VPD identifier or serial collides with another disk",
        )]
    } else {
        vec![]
    }
}

fn layout_snapshot(disk: &PlanningDisk) -> Option<DiskLayoutSnapshot> {
    let mut canonical = disk.clone();
    canonical.identifier.clear();
    canonical.name.clear();
    if let Some(id) = &mut canonical.identity {
        id.unique_id.make_ascii_lowercase();
        id.serial_number.make_ascii_lowercase();
    }
    canonical.geometry.as_mut()?.disk_guid =
        canonical_guid(&canonical.geometry.as_ref()?.disk_guid);
    for p in &mut canonical.partitions {
        p.partition_id = p.partition_id.as_deref().map(canonical_guid);
        p.gpt_type = p.gpt_type.as_deref().map(canonical_guid);
        p.volumes.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        for v in &mut p.volumes {
            v.mount_points.sort();
        }
    }
    canonical
        .partitions
        .sort_by_key(|p| (p.offset_bytes, p.number));
    serde_json::to_string(&canonical)
        .ok()
        .map(|token| DiskLayoutSnapshot { version: 1, token })
}

fn regions(disk: &PlanningDisk, validation: &PlanningValidation) -> Vec<AllocatableRegion> {
    let g = disk.geometry.as_ref().unwrap();
    let identity = disk.identity.as_ref().unwrap();
    let mut partitions: Vec<_> = disk.partitions.iter().collect();
    partitions.sort_by_key(|p| p.offset_bytes);
    let mut cursor = g.usable_start_bytes;
    let mut gaps = Vec::new();
    for p in partitions {
        let start = p.offset_bytes.unwrap();
        if cursor < start {
            gaps.push((cursor, start));
        }
        cursor = start + p.size_bytes.unwrap();
    }
    if cursor < g.usable_end_bytes {
        gaps.push((cursor, g.usable_end_bytes));
    }
    gaps.into_iter()
        .map(|(start, end)| {
            let first = align_up(start, PARTITION_ALIGNMENT_BYTES);
            let last = end - end % PARTITION_ALIGNMENT_BYTES;
            let maximum = first.and_then(|first| last.checked_sub(first)).unwrap_or(0);
            let mut issues = validation.issues.clone();
            if maximum < MIN_PARTITION_BYTES {
                issues.push(issue(
                    PlanningErrorCode::InsufficientAllocatableSpace,
                    "This region is too small after alignment",
                ));
            }
            if disk.partitions.len() >= g.max_partition_count as usize {
                issues.push(issue(
                    PlanningErrorCode::UnsupportedOperation,
                    "The GPT partition table has no available entries",
                ));
            }
            AllocatableRegion {
                parent_identity: identity.clone(),
                identity: AllocatableRegionIdentity {
                    start_offset_bytes: start,
                    end_offset_bytes: end,
                },
                capacity_bytes: end - start,
                required_alignment_bytes: PARTITION_ALIGNMENT_BYTES,
                first_usable_aligned_offset_bytes: first.filter(|first| *first <= last),
                last_usable_aligned_boundary_bytes: (maximum > 0).then_some(last),
                aligned_capacity_bytes: maximum,
                maximum_allocatable_bytes: if issues.is_empty() { maximum } else { 0 },
                partition_table: PartitionScheme::Gpt,
                validation: PlanningValidation::from_issues(issues),
            }
        })
        .collect()
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    let remainder = value % alignment;
    if remainder == 0 {
        Some(value)
    } else {
        value.checked_add(alignment - remainder)
    }
}

pub fn plan_partition_in(
    inventory: &PartitionPlanningInventory,
    request: &PartitionPlanningRequest,
) -> PartitionPlanningResponse {
    use PlanningErrorCode::*;
    if !valid_planning_identifier(request.identifier()) {
        return PartitionPlanningResponse::rejected(issue(
            InvalidParameters,
            "Invalid physical disk identifier",
        ));
    }
    let mut original = inventory
        .disks
        .iter()
        .filter(|d| d.identifier == request.identifier());
    let located = original.next();
    if original.next().is_some() {
        return PartitionPlanningResponse::rejected(issue(
            AmbiguousIdentity,
            "Duplicate physical disk locator",
        ));
    }
    let disk = if let Some((expected, _, _)) = request.expected() {
        if !expected.complete() {
            return PartitionPlanningResponse::rejected(issue(
                MissingMetadata,
                "Expected persistent identity is incomplete",
            ));
        }
        let mut matches = inventory
            .disks
            .iter()
            .filter(|d| d.identity.as_ref().is_some_and(|id| id.matches(expected)));
        let candidate = matches.next();
        if matches.next().is_some() {
            return PartitionPlanningResponse::rejected(issue(
                AmbiguousIdentity,
                "Persistent identity resolves to multiple disks",
            ));
        }
        match candidate {
            Some(disk) => disk,
            None => {
                return PartitionPlanningResponse::rejected(issue(
                    if located.is_some() {
                        StaleDeviceIdentity
                    } else {
                        DeviceDisappeared
                    },
                    "The expected physical disk disappeared or its hardware identity changed",
                ));
            }
        }
    } else if let Some(disk) = located {
        disk
    } else {
        return PartitionPlanningResponse::rejected(issue(
            DeviceDisappeared,
            "The physical disk is missing",
        ));
    };
    let mut geometry_issues = geometry_validation(disk);
    geometry_issues.extend(identity_validation(inventory, disk));
    if disk.device_path.as_ref().is_none_or(|p| p.is_empty()) {
        geometry_issues.push(issue(
            MissingMetadata,
            "An OS device-interface path is required",
        ));
    }
    let geometry_valid = geometry_issues.is_empty();
    let mut all_issues = inventory.issues.clone();
    all_issues.extend(geometry_issues);
    all_issues.extend(partition_safety_policy(disk).issues);
    let validation = PlanningValidation::from_issues(all_issues);
    let snapshot = geometry_valid.then(|| layout_snapshot(disk)).flatten();
    let regions = if geometry_valid {
        regions(disk, &validation)
    } else {
        vec![]
    };
    let deletion_previews: Vec<_> = disk.partitions.iter().map(|p| DeletePartitionPreview {
        parent_disk:disk.identifier.clone(), parent_identity:disk.identity.clone(), partition:p.clone(),
        eligible:validation.valid, validation:validation.clone(),
        data_loss_warning:"Deleting this partition in a future execution milestone would make all data on it inaccessible and may cause permanent data loss. This preview changes no storage.".into(),
    }).collect();
    let discovery = DiskPlanningDiscovery {
        identifier: disk.identifier.clone(),
        name: disk.name.clone(),
        physical_identity: disk.identity.clone(),
        partition_table: disk.partition_table,
        geometry: disk.geometry.clone(),
        layout_snapshot: snapshot.clone(),
        safety: disk.safety.clone(),
        validation: validation.clone(),
        regions,
        partitions: deletion_previews,
    };
    let mut response = PartitionPlanningResponse {
        api_version: 1,
        preview_only: true,
        execution_supported: false,
        validation,
        discovery: Some(discovery),
        create_preview: None,
        delete_preview: None,
    };
    if let Some((expected, expected_layout, test_disk)) = request.expected() {
        if expected_layout.version != 1 || snapshot.as_ref() != Some(expected_layout) {
            response.validation.issues.push(issue(ChangedPartitionLayout, "Disk layout, connection path, safety metadata or associated volumes changed. Discover and preview again."));
        }
        if !test_disk.identity.matches(expected) {
            response.validation.issues.push(issue(
                TestDiskDesignationRequired,
                "Designate this exact persistent identity as an expendable external test disk",
            ));
        }
    }
    match request {
        PartitionPlanningRequest::Discover { .. } => {}
        PartitionPlanningRequest::Create {
            region,
            requested_size_bytes: size,
            requested_start_offset_bytes,
            required_alignment_bytes,
            partition_table,
            proposed_gpt_type,
            ..
        } => {
            if *partition_table != PartitionScheme::Gpt
                || canonical_guid(proposed_gpt_type) != BASIC_DATA_GPT_TYPE
            {
                response.validation.issues.push(issue(
                    UnsupportedOperation,
                    "Only future GPT basic-data partitions can be planned",
                ));
            }
            if *required_alignment_bytes != PARTITION_ALIGNMENT_BYTES
                || *size < MIN_PARTITION_BYTES
                || size % PARTITION_ALIGNMENT_BYTES != 0
            {
                response.validation.issues.push(issue(InvalidParameters, "Size must be at least 1 MiB and an exact multiple of the required 1 MiB alignment"));
            }
            let selected = response
                .discovery
                .as_ref()
                .unwrap()
                .regions
                .iter()
                .find(|r| r.identity == *region);
            if let Some(r) = selected {
                response.validation.issues.extend(
                    r.validation
                        .issues
                        .iter()
                        .filter(|i| !response.validation.issues.contains(i))
                        .cloned()
                        .collect::<Vec<_>>(),
                );
                if let (Some(first), Some(last)) = (
                    r.first_usable_aligned_offset_bytes,
                    r.last_usable_aligned_boundary_bytes,
                ) {
                    let start = requested_start_offset_bytes.unwrap_or(first);
                    if start % PARTITION_ALIGNMENT_BYTES != 0 || start < first || start >= last {
                        response.validation.issues.push(issue(
                            InvalidParameters,
                            "Requested start is outside the aligned region or is misaligned",
                        ));
                    }
                    let end = start.checked_add(*size);
                    if end.is_none_or(|end| end > last) {
                        response.validation.issues.push(issue(
                            InsufficientAllocatableSpace,
                            "Requested allocation overflows or exceeds the selected region",
                        ));
                    }
                    if response.validation.issues.is_empty() {
                        let end = end.unwrap();
                        response.create_preview = Some(CreatePartitionPreview {
                            parent_identity: disk.identity.clone().unwrap(),
                            region: region.clone(),
                            expected_layout: snapshot.unwrap(),
                            partition_table: PartitionScheme::Gpt,
                            proposed_gpt_type: BASIC_DATA_GPT_TYPE.into(),
                            required_alignment_bytes: PARTITION_ALIGNMENT_BYTES,
                            start_offset_bytes: start,
                            end_offset_bytes: end,
                            allocated_size_bytes: *size,
                            remaining_region_capacity_bytes: r.capacity_bytes - size,
                            remaining_allocatable_capacity_bytes: (start - first) + (last - end),
                        });
                    }
                }
            } else {
                response.validation.issues.push(issue(
                    ChangedPartitionLayout,
                    "The selected allocatable region does not exist in this layout",
                ));
            }
        }
        PartitionPlanningRequest::Delete { partition, .. } => {
            let selected = disk.partitions.iter().position(|p| {
                p.partition_id
                    .as_ref()
                    .is_some_and(|id| canonical_guid(id) == canonical_guid(&partition.partition_id))
            });
            if let Some(index) = selected {
                let p = &disk.partitions[index];
                if p.number != Some(partition.number)
                    || p.offset_bytes != Some(partition.offset_bytes)
                    || p.size_bytes != Some(partition.size_bytes)
                    || p.gpt_type.as_deref().map(canonical_guid)
                        != Some(canonical_guid(&partition.gpt_type))
                {
                    response.validation.issues.push(issue(
                        ChangedPartitionLayout,
                        "Partition number, extent or type changed",
                    ));
                }
                let mut preview = response.discovery.as_ref().unwrap().partitions[index].clone();
                preview.validation =
                    PlanningValidation::from_issues(response.validation.issues.clone());
                preview.eligible = preview.validation.valid;
                response.delete_preview = Some(preview);
            } else {
                response.validation.issues.push(issue(
                    ChangedPartitionLayout,
                    "The selected partition GUID is absent from the current disk",
                ));
            }
        }
    }
    response.validation.valid = response.validation.issues.is_empty();
    response
}
