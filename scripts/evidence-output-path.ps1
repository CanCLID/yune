function Invoke-YuneEvidenceOutputPathPolicy([string[]]$Arguments) {
    $Tool = Join-Path $PSScriptRoot "evidence-output-path.py"
    if (-not (Test-Path -LiteralPath $Tool -PathType Leaf)) {
        throw "Missing evidence output path policy: $Tool"
    }

    $Python = $null
    $Prefix = @()
    $Candidates = @(
        [pscustomobject]@{ Name = "python3"; Prefix = @() },
        [pscustomobject]@{ Name = "py"; Prefix = @("-3") },
        [pscustomobject]@{ Name = "python"; Prefix = @() }
    )
    foreach ($Candidate in $Candidates) {
        $Command = Get-Command $Candidate.Name -ErrorAction SilentlyContinue
        if ($null -eq $Command) {
            continue
        }
        & $Command.Source @($Candidate.Prefix) -B -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 9) else 1)" 2>$null
        if ($LASTEXITCODE -eq 0) {
            $Python = $Command
            $Prefix = @($Candidate.Prefix)
            break
        }
    }
    if ($null -eq $Python) {
        throw "Python 3.9 or newer is required for the evidence output path policy"
    }

    $Output = @(& $Python.Source @Prefix -B $Tool @Arguments 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0) {
        throw "Evidence output path policy rejected the destination: $($Output -join ' | ')"
    }
    if ($Output.Count -ne 1 -or [string]::IsNullOrWhiteSpace($Output[0])) {
        throw "Evidence output path policy returned an invalid path: $($Output -join ' | ')"
    }
    return $Output[0]
}
