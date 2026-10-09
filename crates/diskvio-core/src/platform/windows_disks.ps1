$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Import-Module Storage -ErrorAction Stop
$disks = @(Get-Disk -ErrorAction Stop | Sort-Object Number | ForEach-Object {
    [PSCustomObject]@{
        number = [uint32]$_.Number
        name = [string]$_.FriendlyName
        size_bytes = [uint64]$_.Size
        bus_type = [string]$_.BusType
        partition_style = [string]$_.PartitionStyle
    }
})
ConvertTo-Json -InputObject $disks -Depth 3 -Compress
