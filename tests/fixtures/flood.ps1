# The OUT-005 test load: 50,000 lines as fast as PowerShell can write them, every tenth line on
# stderr.
$text = 'x' * 80
for ($i = 1; $i -le 50000; $i++) {
    if ($i % 10 -eq 0) {
        [Console]::Error.WriteLine("err $i $text")
    }
    else {
        [Console]::Out.WriteLine("out $i $text")
    }
}
