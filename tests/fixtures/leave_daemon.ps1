# Starts a hidden grandchild that sleeps, reports its process id, then exits at once: the shape
# of a build that leaves a compiler server or build daemon running.
$info = [System.Diagnostics.ProcessStartInfo]::new('powershell.exe', '-NoProfile -Command Start-Sleep -Seconds 60')
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$child = [System.Diagnostics.Process]::Start($info)
Write-Output "child:$($child.Id)"
exit 0
