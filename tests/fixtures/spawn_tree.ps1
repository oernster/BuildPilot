# Starts a hidden grandchild that sleeps, reports its process id, then sleeps itself.
$info = [System.Diagnostics.ProcessStartInfo]::new('powershell.exe', '-NoProfile -Command Start-Sleep -Seconds 60')
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$child = [System.Diagnostics.Process]::Start($info)
Write-Output "child:$($child.Id)"
Start-Sleep -Seconds 60
