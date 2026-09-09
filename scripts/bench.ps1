# Windows port of scripts/bench.sh: time the plain step() loop over ZEXALL,
# pinned to one logical CPU, under the same three code layouts, and report
# each plus the minimum.
#
# Two differences from bench.sh, both forced by the platform:
#   - pinning is `start /affinity <hexmask>`, which sets the mask before the
#     process runs, in place of taskset;
#   - the default toolchain is x86_64-pc-windows-gnu, which rustup links
#     without an external gcc; the msvc target needs Visual Studio's link.exe.
#
# Usage: scripts/bench.ps1 [-Cpu 4] [-Runs 1] [-Layouts default,noalign,nf5]
#                          [-Root <checkout>] [-Manifest <path>]
#                          [-Toolchain <name>] [-BenchArgs '--limit N']
param(
    [int]$Cpu = 4,
    [int]$Runs = 1,
    [string[]]$Layouts = @('default', 'noalign', 'nf5'),
    [string]$Root = (Resolve-Path "$PSScriptRoot\..").Path,
    [string]$Manifest = 'conformance/zex/zexall.json',
    [string]$Toolchain = 'stable-x86_64-pc-windows-gnu',
    [string]$BenchArgs = ''
)
$ErrorActionPreference = 'Stop'
# Setting RUSTFLAGS replaces .cargo/config.toml's rustflags entirely, which is
# what "noalign" relies on; "default" leaves it unset so the config applies.
$flags = @{
    noalign = '-C opt-level=3'
    nf5     = '-C llvm-args=-align-all-nofallthru-blocks=5'
    nf6     = '-C llvm-args=-align-all-nofallthru-blocks=6'
}
$mask = '{0:x}' -f ([int64]1 -shl $Cpu)
$dirty = if ((git -C $Root status --porcelain -- . ':!README.md' ':!docs')) { ' (dirty)' } else { '' }
"commit $(git -C $Root rev-parse --short HEAD)$dirty, cpu $Cpu (mask 0x$mask), $Toolchain, rustc $((rustc +$Toolchain --version) -split ' ' | Select-Object -Index 1)"
$best = [double]::PositiveInfinity
$bestLayout = ''
foreach ($layout in $Layouts) {
    if ($layout -eq 'default') {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTFLAGS = $flags[$layout]
    }
    $env:CARGO_TARGET_DIR = "$Root\target\win-$layout"
    cargo "+$Toolchain" build --release -q --manifest-path "$Root\Cargo.toml"
    $bin = "$Root\target\win-$layout\release\z80-bench.exe"
    Remove-Item Env:RUSTFLAGS, Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    for ($i = 0; $i -lt $Runs; $i++) {
        $out = [System.IO.Path]::GetTempFileName()
        cmd /c "start `"`" /affinity $mask /b /wait `"$bin`" `"$Root\$Manifest`" $BenchArgs > `"$out`" 2>&1"
        $line = (Get-Content $out -Raw).Trim()
        Remove-Item $out
        "${layout}: $line"
        if ($line -match ', ([0-9.]+) s,') {
            $seconds = [double]$Matches[1]
            if ($seconds -lt $best) { $best = $seconds; $bestLayout = $layout }
        }
    }
}
"min over layouts: $best s ($bestLayout)"
