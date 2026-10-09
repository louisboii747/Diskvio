$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Import-Module Storage -ErrorAction Stop
$warnings = [System.Collections.Generic.List[string]]::new()
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
    }
}
$disks = @(Get-Disk -ErrorAction Stop | Sort-Object Number | ForEach-Object {
    $d = $_
    $partitions = @()
    try {
        $partitions = @(Get-Partition -DiskNumber $d.Number -ErrorAction Stop | Sort-Object PartitionNumber | ForEach-Object {
            $p = $_
            $volumes = @()
            try {
                $volumes = @(Get-Volume -Partition $p -ErrorAction Stop | ForEach-Object {
                    [void]$associatedIds.Add([string]$_.Path)
                    Convert-Volume $_
                })
            } catch {
                if ($p.Type -eq 'Basic' -or $p.Type -eq 'IFS' -or $p.DriveLetter) {
                    $warnings.Add("Disk $($d.Number) partition $($p.PartitionNumber) volumes: $($_.Exception.Message)")
                }
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
                volumes = $volumes
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
        serial_number = [string]$d.SerialNumber
        path = [string]$d.Path
        guid = [string]$d.Guid
        signature = if ($d.PartitionStyle -eq 'MBR') { [uint32]$d.Signature } else { $null }
        system = $d.IsSystem
        boot = $d.IsBoot
        offline = $d.IsOffline
        read_only = $d.IsReadOnly
        clustered = $d.IsClustered
        partitions = $partitions
    }
})
$unattached = @($allVolumes | Where-Object { -not $associatedIds.Contains([string]$_.Path) } | ForEach-Object { Convert-Volume $_ })
[PSCustomObject]@{ disks = $disks; unattached_volumes = $unattached; warnings = @($warnings.ToArray()) } | ConvertTo-Json -Depth 8 -Compress
