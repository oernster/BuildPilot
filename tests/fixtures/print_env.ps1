# Prints the variables an activated step depends on, one per line, then exits 0.
# pwsh puts its own folder ($PSHOME) first on PATH as it starts (measured 2026-09-27), so the
# first entry after it is the one BuildPilot placed.
Write-Output "VIRTUAL_ENV:$env:VIRTUAL_ENV"
Write-Output "PATH0:$(@($env:PATH -split ';' | Where-Object { $_ -ne $PSHOME })[0])"
Write-Output "PYTHON:$(@(Get-Command python.exe -CommandType Application)[0].Source)"
Write-Output "PYTHONIOENCODING:$env:PYTHONIOENCODING"
Write-Output "PYTHONUNBUFFERED:$env:PYTHONUNBUFFERED"
Write-Output "PATHS:$(@(Get-ChildItem env: | Where-Object Name -eq 'PATH').Count)"
exit 0
