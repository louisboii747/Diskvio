$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Import-Module Storage -ErrorAction Stop
$warnings = [System.Collections.Generic.List[string]]::new()
$planningWarnings = [System.Collections.Generic.List[string]]::new()
$physicalDisks = @()
$encryptableVolumes = @()
if ($DiskvioPartitionPlanning) {
    try { $physicalDisks = @(Get-PhysicalDisk -ErrorAction Stop) }
    catch { $planningWarnings.Add("Physical storage safety discovery: $($_.Exception.Message)") }
    try { $encryptableVolumes = @(Get-CimInstance -Namespace root/CIMV2/Security/MicrosoftVolumeEncryption -ClassName Win32_EncryptableVolume -ErrorAction Stop) }
    catch { $encryptableVolumes = @() }
}
$pageFiles = @()
try { $pageFiles = @(Get-CimInstance Win32_PageFileUsage -ErrorAction Stop | ForEach-Object { $_.Name }) }
catch { $warnings.Add("Paging-file safety discovery: $($_.Exception.Message)") }
$allVolumes = @(Get-Volume -ErrorAction Stop)
$associatedIds = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
function Convert-Volume($v) {
    [PSCustomObject]@{
        path = [string]$v.Path
        unique_id = [string]$v.UniqueId
        drive_letter = if ($v.DriveLetter -and [int][char]$v.DriveLetter -ne 0) { [string]$v.DriveLetter } else { $null }
        label = [string]$v.FileSystemLabel
        filesystem = [string]$v.FileSystemType
        size_bytes = [uint64]$v.Size
        available_bytes = [uint64]$v.SizeRemaining
        health_status = [string]$v.HealthStatus
        operational_status = @($v.OperationalStatus | ForEach-Object { [string]$_ })
    }
}
$disks = @(Get-Disk -ErrorAction Stop | Sort-Object Number | ForEach-Object {
    $d = $_
    $diskPlanning = $null
    if ($DiskvioPartitionPlanning) {
        $pool = $null
        $physical = @($physicalDisks | Where-Object { $_.UniqueId -and $_.SerialNumber -and $_.UniqueId.Trim() -eq $d.UniqueId.Trim() -and $_.SerialNumber.Trim() -eq $d.SerialNumber.Trim() })
        if ($physical.Count -eq 1) {
            try { $pool = @(Get-StoragePool -PhysicalDisk $physical[0] -ErrorAction Stop | Where-Object { -not $_.IsPrimordial }).Count -gt 0 }
            catch { $planningWarnings.Add("Disk $($d.Number) storage pool safety: $($_.Exception.Message)") }
        }
        $diskPlanning = [PSCustomObject]@{
            boot_from_disk = $d.BootFromDisk
            storage_pool = $pool
            logical_sector_bytes = $d.LogicalSectorSize
            physical_sector_bytes = $d.PhysicalSectorSize
            partition_count = $d.NumberOfPartitions
        }
    }
    $partitions = @()
    try {
        $partitions = @(Get-Partition -DiskNumber $d.Number -ErrorAction Stop | Sort-Object PartitionNumber | ForEach-Object {
            $p = $_
            $volumes = @()
            $volumesComplete = $true
            try {
                $volumes = @(Get-Volume -Partition $p -ErrorAction Stop | ForEach-Object {
                    [void]$associatedIds.Add([string]$_.Path)
                    Convert-Volume $_
                })
            } catch {
                $volumesComplete = $false
                if ($p.Type -eq 'Basic' -or $p.Type -eq 'IFS' -or $p.DriveLetter) {
                    $warnings.Add("Disk $($d.Number) partition $($p.PartitionNumber) volumes: $($_.Exception.Message)")
                }
            }
            $partitionPlanning = $null
            if ($DiskvioPartitionPlanning) {
                $encrypted = $null
                $hostsWindows = $null
                if ($volumesComplete -and $volumes.Count -gt 0) {
                    try {
                        $bootStates = @($volumes | ForEach-Object {
                            $root = [string]$_.path
                            if (-not $root) { throw 'Volume root is unavailable' }
                            $null = Get-Item -LiteralPath $root -Force -ErrorAction Stop
                            (Test-Path -LiteralPath ($root + 'Windows') -ErrorAction Stop) -or
                            (Test-Path -LiteralPath ($root + 'Boot') -ErrorAction Stop) -or
                            (Test-Path -LiteralPath ($root + 'EFI') -ErrorAction Stop)
                        })
                        if ($bootStates.Count -eq $volumes.Count) { $hostsWindows = @($bootStates | Where-Object { $_ -eq $true }).Count -gt 0 }
                    } catch { $hostsWindows = $null }
                    $states = @($volumes | ForEach-Object {
                        $volume = $_
                        $matches = @($encryptableVolumes | Where-Object { $_.DeviceID -eq $volume.path })
                        if ($matches.Count -ne 1) { $null }
                        else {
                            try {
                                $protection = Invoke-CimMethod -InputObject $matches[0] -MethodName GetProtectionStatus -ErrorAction Stop
                                $conversion = Invoke-CimMethod -InputObject $matches[0] -MethodName GetConversionStatus -ErrorAction Stop
                                $lock = Invoke-CimMethod -InputObject $matches[0] -MethodName GetLockStatus -ErrorAction Stop
                                if ($protection.ReturnValue -ne 0 -or $conversion.ReturnValue -ne 0 -or $lock.ReturnValue -ne 0) { $null }
                                else { $protection.ProtectionStatus -ne 0 -or $conversion.ConversionStatus -ne 0 -or $lock.LockStatus -ne 0 }
                            } catch { $null }
                        }
                    })
                    if ($states.Count -eq $volumes.Count -and @($states | Where-Object { $null -eq $_ }).Count -eq 0) {
                        $encrypted = @($states | Where-Object { $_ -eq $true }).Count -gt 0
                    }
                }
                $partitionPlanning = [PSCustomObject]@{ encrypted = $encrypted; hosts_windows = $hostsWindows; volumes_complete = $volumesComplete }
            }
            [PSCustomObject]@{
                number = [uint32]$p.PartitionNumber
                guid = [string]$p.Guid
                gpt_type = [string]$p.GptType
                mbr_type = if ($d.PartitionStyle -eq 'MBR') { [uint16]$p.MbrType } else { $null }
                type = [string]$p.Type
                offset_bytes = [uint64]$p.Offset
                size_bytes = [uint64]$p.Size
                access_paths = @($p.AccessPaths | Where-Object { $_ })
                system = $p.IsSystem
                boot = $p.IsBoot
                active = $p.IsActive
                hidden = $p.IsHidden
                read_only = $p.IsReadOnly
                offline = $p.IsOffline
                shadow_copy = $p.IsShadowCopy
                no_default_drive_letter = $p.NoDefaultDriveLetter
                page_file = @($p.AccessPaths | Where-Object {
                    $accessPath = $_
                    @($pageFiles | Where-Object { $_.StartsWith($accessPath, [System.StringComparison]::OrdinalIgnoreCase) }).Count -gt 0
                }).Count -gt 0
                volumes = $volumes
                planning = $partitionPlanning
            }
        })
    } catch {
        $warnings.Add("Disk $($d.Number) partitions: $($_.Exception.Message)")
    }
    [PSCustomObject]@{
        number = [uint32]$d.Number
        name = [string]$d.FriendlyName
        size_bytes = [uint64]$d.Size
        bus_type = [string]$d.BusType
        partition_style = [string]$d.PartitionStyle
        unique_id = [string]$d.UniqueId
        unique_id_format = $d.UniqueIdFormat
        serial_number = [string]$d.SerialNumber
        path = [string]$d.Path
        guid = [string]$d.Guid
        signature = if ($d.PartitionStyle -eq 'MBR') { [uint32]$d.Signature } else { $null }
        system = $d.IsSystem
        boot = $d.IsBoot
        offline = $d.IsOffline
        read_only = $d.IsReadOnly
        clustered = $d.IsClustered
        health_status = [string]$d.HealthStatus
        operational_status = @($d.OperationalStatus | ForEach-Object { [string]$_ })
        partitions = $partitions
        planning = $diskPlanning
    }
})
$unattached = @($allVolumes | Where-Object { -not $associatedIds.Contains([string]$_.Path) } | ForEach-Object { Convert-Volume $_ })
[PSCustomObject]@{ disks = $disks; unattached_volumes = $unattached; warnings = @($warnings.ToArray()); planning_warnings = @($planningWarnings.ToArray()) } | ConvertTo-Json -Depth 8 -Compress
