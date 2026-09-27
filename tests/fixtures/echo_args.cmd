@echo off
rem Prints each argument (outer quotes removed) and the working directory, writes one line to
rem stderr, exits 4.
:next
if [%1]==[] goto done
echo arg:%~1
shift
goto next
:done
echo cwd:%CD%
echo to stderr 1>&2
exit /b 4
