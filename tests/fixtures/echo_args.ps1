# Prints each argument and the working directory, writes one line to stderr, exits 3.
foreach ($argument in $args) { Write-Output "arg:$argument" }
Write-Output "cwd:$((Get-Location).Path)"
[Console]::Error.WriteLine('to stderr')
exit 3
