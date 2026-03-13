# Testing Strategy for Linux Binary Compatibility

This document lays out a practical plan for integrating established Linux test suites into Solaya to measure and drive progress toward full binary compatibility with Linux.

## Current State

For current syscall and subsystem details, see `doc/ai/SYSCALLS.md` and `doc/ai/TESTING.md`. The existing system test infrastructure boots QEMU per test and interacts via serial. Userspace binaries are statically linked against musl (riscv64) and embedded in the kernel. VirtIO block is supported and used for ext2 tests, providing a path to load external test binaries.

---

## 1. Test Suite Integration

### 1.1 libc-test (musl's own test suite) -- FIRST PRIORITY

**What it is.** The official correctness test suite for musl libc, maintained by Szabolcs Nagy. It tests the C library interface that every userspace program depends on. Since Solaya uses musl, this is the most direct measure of whether the kernel provides the syscall surface that musl expects.

**Structure.** The source lives in five directories under `src/`:

| Directory | Purpose | Approx. tests |
|-----------|---------|---------------|
| `src/api/` | Header conformance -- checks that headers declare the right types, macros, and function prototypes | ~80 |
| `src/functional/` | Functional tests for libc operations (fork, exec, pipes, mmap, signals, threads, stdio, etc.) | ~80 |
| `src/math/` | Math function accuracy with input-output test vectors | ~80 |
| `src/regression/` | Regression tests for specific musl bugs (each references a commit or bug ID) | ~50 |
| `src/common/` | Shared test utilities (`libtest.a` with `t_error()` and `t_status`) | N/A (support code) |

Total: roughly 300 tests. Each `.c` file produces both a dynamic and static binary.

**Cross-compilation for riscv64-linux-musl.**

```bash
git clone https://repo.or.cz/libc-test.git
cd libc-test
cp config.mak.def config.mak
```

Edit `config.mak`:
```makefile
CC = riscv64-unknown-linux-musl-gcc
CFLAGS += -static
# Disable dynamic tests (Solaya has no dynamic linker)
# Build only static variants
```

Then `make` builds all tests and produces a `REPORT` file.

**Output format.** Tests follow a simple convention: exit 0 = pass, non-zero = fail. Failing tests print diagnostics via `t_error()` to stdout. The `REPORT` file aggregates all build errors and runtime failures. Successful tests produce no output.

**How to run on Solaya.** Each test is a standalone statically-linked ELF binary. The plan:

1. Cross-compile the full suite with `CC=riscv64-unknown-linux-musl-gcc CFLAGS="-static"`.
2. Pack all test binaries into an ext2 disk image (Solaya already supports VirtIO block + ext2).
3. Write a small test runner program (in Rust, compiled into userspace) that:
   - Iterates over test binaries in `/mnt/tests/`
   - fork+exec each one
   - Captures exit code
   - Prints `PASS <name>` or `FAIL <name>` to serial
4. The system test on the host side boots QEMU with the test disk image, runs the test runner, and parses the serial output.

**Integration difficulty: LOW.** These are simple static binaries. The main challenges will be missing syscalls causing tests to crash. That is the point: each crash identifies the next syscall to implement.

**Reference.** rCore-OS (a RISC-V teaching OS in Rust) maintains a fork at `github.com/rcore-os/libc-test` showing this exact approach works.

---

### 1.2 LTP (Linux Test Project) -- SECOND PRIORITY

**What it is.** The primary Linux kernel compliance test suite, maintained by SUSE, Red Hat, Fujitsu, IBM, and others. It contains 3800+ tests covering syscalls, memory management, IPC, scheduling, filesystems, containers, networking, and POSIX conformance.

**Structure.**

```
ltp/
  testcases/
    kernel/
      syscalls/       # ~376 directories, one per syscall (fork, open, mmap, ...)
      mem/            # Memory management tests
      ipc/            # IPC (shared memory, semaphores, message queues)
      sched/          # Scheduler tests
      io/             # I/O tests
      containers/     # Namespace/cgroup tests
      fs/             # Filesystem tests
      ...
    open_posix_testsuite/  # POSIX conformance tests (~1700 tests)
    network/               # Network tests (~1000 tests)
  runtest/                 # Test lists (scenario files)
    syscalls              # Lists all syscall tests
    mm                    # Memory management tests
    fs                    # Filesystem tests
    io                    # I/O tests
    ipc                   # IPC tests
    sched                 # Scheduler tests
    ...
  lib/                    # LTP test library (libltp.a)
  include/                # LTP headers (tst_test.h, etc.)
```

**Runtest file format.** Each line in a runtest file is:
```
test_name    /path/to/binary [args]
```
Example from `runtest/syscalls`:
```
fork01       fork01
fork02       fork02
open01       open01
mmap01       mmap01
```
The first column is the test ID (appears in logs), the rest is the command to execute.

**Test output format.** LTP tests use standardized result macros:

| Macro | Meaning |
|-------|---------|
| `TPASS` | Test passed |
| `TFAIL` | Test failed (unexpected result) |
| `TCONF` | Test not applicable (missing feature/config) |
| `TBROK` | Test broken (setup failure, not a kernel bug) |
| `TWARN` | Warning (side effect, not a failure) |
| `TINFO` | Informational message |

Output line format: `<filename>:<line>: <RESULT>: <message>`

Example: `fork01.c:62: TPASS: fork() returned child pid`

Summary line: `Summary: passed 2 failed 0 broken 0 skipped 0 warnings 0`

Exit code encodes results as bit flags: TFAIL=bit 1, TBROK=bit 2, TWARN=bit 4, TCONF=bit 32.

**Cross-compilation for riscv64-linux-musl.**

```bash
git clone https://github.com/linux-test-project/ltp.git
cd ltp
make autotools
./configure \
    CC=riscv64-unknown-linux-musl-gcc \
    AR=riscv64-unknown-linux-musl-ar \
    RANLIB=riscv64-unknown-linux-musl-ranlib \
    STRIP=riscv64-unknown-linux-musl-strip \
    --host=riscv64-linux-musl \
    --prefix=/opt/ltp \
    CFLAGS="-static -D_GNU_SOURCE" \
    LDFLAGS="-static -pthread"
make -j$(nproc)
make install DESTDIR=$(pwd)/install
```

**Musl compatibility notes.** LTP was designed for glibc. Some tests use glibc-specific extensions. The LTP CI has a list of tests that must be excluded when building with musl (they fail to compile). Expect ~5-10% of tests to need exclusion or patching. Common issues:
- `__off64_t` vs `off_t` (musl uses 64-bit off_t everywhere)
- Missing `_LARGEFILE64_SOURCE` functions
- POSIX features musl intentionally omits (e.g., `wordexp` with `WRDE_CMDSUB`)

**How to run on Solaya.**

1. Cross-compile LTP with static linking.
2. Package the `install/opt/ltp/testcases/bin/` directory into a large ext2 image (hundreds of MB).
3. Start with individual tests: `fork01`, `open01`, `read01`, `write01`, etc.
4. Use the same test-runner approach as libc-test.

**Integration difficulty: MEDIUM.** LTP tests are more complex than libc-test. Many tests:
- Require `/tmp`, `/proc`, and other filesystem layout
- Use features like POSIX shared memory, IPC, or cgroups
- Depend on having `root` privileges
- Assume Linux-specific `/proc` entries exist

The strategy is to cherry-pick individual syscall tests (the ~376 tests in `testcases/kernel/syscalls/`) first, since those are the most useful for measuring compatibility. Each syscall directory has numbered variants (e.g., `fork01`, `fork02`, ..., `fork13`) testing different aspects.

---

### 1.3 Linux kselftest -- THIRD PRIORITY

**What it is.** The kernel's own self-test suite, located in `tools/testing/selftests/` within the Linux kernel source tree. Contains 132+ subdirectories covering kernel subsystems.

**Relevant subdirectories for Solaya (ordered by priority).**

| Directory | Relevance | Why |
|-----------|-----------|-----|
| `signal/` | HIGH | Signal delivery, masking, SA_SIGINFO |
| `timers/` | HIGH | POSIX timers, nanosleep, clock_gettime |
| `futex/` | HIGH | Futex operations (musl threading depends on these) |
| `clone3/` | HIGH | clone3 syscall (modern thread creation) |
| `pidfd/` | MEDIUM | pidfd_open, pidfd_send_signal |
| `exec/` | MEDIUM | execve edge cases |
| `rlimits/` | MEDIUM | Resource limits |
| `mm/` | MEDIUM | mmap, mprotect, brk edge cases |
| `mqueue/` | MEDIUM | POSIX message queues |
| `ipc/` | MEDIUM | System V IPC |
| `namespaces/` | LOW | Requires namespace support |
| `net/` | LOW | Complex network tests |
| `seccomp/` | LOW | Not needed for basic compatibility |
| `bpf/` | LOW | Not needed for basic compatibility |
| `riscv/` | MEDIUM | RISC-V specific tests (hwprobe, etc.) |
| `vDSO/` | MEDIUM | Virtual DSO tests |

**Cross-compilation.**

```bash
# From Linux kernel source tree
make -C tools/testing/selftests \
    ARCH=riscv \
    CROSS_COMPILE=riscv64-unknown-linux-musl- \
    TARGETS="signal timers futex clone3 pidfd exec" \
    KSFT_INSTALL_PATH=$(pwd)/kselftest-install \
    install
```

**Output format.** Uses TAP (Test Anything Protocol):
```
TAP version 13
1..4
ok 1 test_signal_delivery
not ok 2 test_signal_mask
ok 3 test_timer_create
ok 4 # SKIP test_timer_overrun: not supported
```

**Integration difficulty: MEDIUM-HIGH.** Many kselftest tests assume a full Linux kernel with specific `/proc` and `/sys` entries. However, the signal, timer, and futex tests are mostly self-contained and very useful.

---

### 1.4 stress-ng -- FOURTH PRIORITY

**What it is.** A comprehensive stress testing tool that can exercise 300+ kernel subsystems through "stressors" organized into categories: cpu, memory, filesystem, network, signal, scheduler, pipe, ipc, io, and more.

**Cross-compilation.**

```bash
git clone https://github.com/ColinIanKing/stress-ng.git
cd stress-ng
CC=riscv64-unknown-linux-musl-gcc \
CXX=riscv64-unknown-linux-musl-g++ \
STATIC=1 \
make -j$(nproc)
```

stress-ng explicitly supports static musl builds and has been tested on RISC-V.

**How to use.** Unlike the compliance test suites, stress-ng is a single binary. It can be embedded directly or loaded via block device. Usage:

```bash
# Stress fork/exec for 10 seconds
stress-ng --fork 4 --timeout 10
# Stress signal handling
stress-ng --signal 4 --timeout 10
# Stress mmap
stress-ng --mmap 4 --timeout 10
```

**Value for Solaya.** stress-ng will not tell you whether a syscall is correctly implemented -- it tells you whether the kernel survives under load. It exposes race conditions, deadlocks, memory leaks, and resource exhaustion bugs that unit tests miss. It is a complement to LTP, not a replacement.

**Integration difficulty: LOW-MEDIUM.** It is a single binary. The difficulty is that stress-ng exercises many syscalls, so it will not work at all until a certain baseline is reached. Start with individual stressors (`--fork`, `--pipe`, `--signal`) once the relevant syscalls work.

---

### 1.5 syzkaller -- FUTURE (NOT NOW)

**What it is.** Google's coverage-guided kernel fuzzer. It generates random sequences of syscalls to find bugs (crashes, hangs, memory corruption). It supports Linux on riscv64.

**Architecture.**

```
syz-manager (host)        Orchestrates fuzzing, manages VMs
    |
    +-- syz-fuzzer (VM)   Generates and mutates test programs
         |
         +-- syz-executor (VM)  Executes syscall sequences, collects coverage
```

**Why NOT now.** syzkaller requires:
- **KCOV** kernel support for code coverage (Solaya would need to implement this)
- **SSH access** to the VM (syz-manager connects via SSH to start syz-fuzzer)
- **A reasonably complete syscall surface** -- fuzzing a kernel with 74 syscalls will just crash on every unknown syscall
- **Crash symbolization** -- syz-manager needs to read kernel symbols from crash dumps
- **Custom OS porting** -- adding a new OS to syzkaller requires implementing OS-specific support in Go: VM management, crash extraction, image building. The NetBSD port took months of GSoC effort.

**When to consider it.** After Solaya passes >50% of LTP syscall tests and has stable process management. Even then, the investment is large. A simpler approach would be to write a custom fuzzer that generates random syscall sequences using Solaya's existing syscall number table. This would catch many of the same bugs without the porting effort.

**Integration difficulty: VERY HIGH.** Months of work. Defer until the kernel is much more mature.

---

## 2. Compliance Tracking

### 2.1 Compliance Matrix

Track compliance as a machine-readable data file that is updated automatically.

**File: `compliance/results.json`**

```json
{
  "generated": "2026-03-13T12:00:00Z",
  "kernel_commit": "abc1234",
  "suites": {
    "libc-test": {
      "total": 290,
      "pass": 142,
      "fail": 98,
      "skip": 50,
      "tests": {
        "src/functional/fork": "pass",
        "src/functional/mmap": "fail",
        "src/functional/pthread": "fail",
        "src/math/sin": "pass"
      }
    },
    "ltp-syscalls": {
      "total": 1200,
      "pass": 340,
      "fail": 710,
      "skip": 150,
      "tests": {
        "fork01": "pass",
        "fork02": "pass",
        "fork03": "fail"
      }
    }
  }
}
```

### 2.2 Generated Dashboard

A script reads `results.json` and generates `compliance/DASHBOARD.md`:

```markdown
# Solaya Compliance Dashboard
Generated: 2026-03-13 | Kernel: abc1234

## Summary
| Suite | Pass | Fail | Skip | Total | Rate |
|-------|------|------|------|-------|------|
| libc-test | 142 | 98 | 50 | 290 | 49% |
| LTP syscalls | 340 | 710 | 150 | 1200 | 28% |

## Recent Progress
- +12 tests since last run (fork03, pipe01, pipe02, ...)

## Failing Tests by Category
### Missing syscalls (most common failure cause)
- `src/functional/pthread`: needs `clone` with `CLONE_THREAD`
- `fork03`: needs `waitpid` (currently only `wait4`)
```

### 2.3 CI Artifact

The compliance run produces `results.json` as a CI artifact. The dashboard can be:
- A generated markdown file committed to the repo (simplest)
- A GitHub Actions artifact downloadable from the CI run
- A GitHub Pages site generated from the JSON (nicest, but more setup)

Recommendation: start with a committed markdown file. Automate with a CI job that runs the compliance suite, generates the dashboard, and opens a PR if results changed. This makes progress visible in every PR review.

---

## 3. Test Infrastructure

### 3.1 Loading Test Binaries from VirtIO Block Device

The current approach of embedding userspace binaries in the kernel works for small programs but cannot scale to thousands of LTP test binaries (hundreds of MB). The infrastructure for block device loading already exists:

1. **Build time:** A script creates an ext2 image containing all test binaries.
2. **QEMU launch:** The image is attached via `--block` (already supported in `qemu_wrapper.sh`).
3. **Inside Solaya:** The ext2 filesystem is mounted at `/mnt` (already implemented).
4. **Test binaries live at** `/mnt/tests/libc-test/`, `/mnt/tests/ltp/`, etc.

**Build script (`scripts/build-test-image.sh`):**

```bash
#!/bin/bash
set -e
IMAGE="$1"
TESTS_DIR="$2"
SIZE_MB="${3:-256}"

# Create empty image
dd if=/dev/zero of="$IMAGE" bs=1M count="$SIZE_MB"
mkfs.ext2 -d "$TESTS_DIR" -F "$IMAGE"
```

### 3.2 In-Kernel Test Harness

A userspace program (`compliance-runner`) runs inside Solaya and reports results over serial. This avoids the overhead of the host-side `ReadAsserter` parsing complex output from thousands of tests.

```
compliance-runner architecture:
  1. Read test list from /mnt/tests/manifest.txt
  2. For each test:
     a. fork()
     b. In child: exec(test_binary)
     c. In parent: waitpid() with timeout
     d. Classify result: PASS (exit 0), FAIL (exit != 0), TIMEOUT, CRASH (signal)
     e. Print structured line: "RESULT <suite> <test_name> <status> <exit_code> <elapsed_ms>"
  3. Print summary: "SUMMARY <suite> <pass> <fail> <timeout> <crash>"
```

The host-side system test simply:
1. Boots QEMU with the test disk image
2. Runs `compliance-runner`
3. Reads `RESULT` lines from serial
4. Parses them into `results.json`

**Why not run LTP's own `runltp` script?** Because `runltp` is a bash script that requires many utilities Solaya does not have (awk, sed, grep, etc.), plus it uses the `ltp-pan` test driver which has its own dependencies. A purpose-built runner in Rust or C is simpler and gives us exact control over timeout handling and result format.

### 3.3 Parallel Test Execution

For a single QEMU instance, tests run sequentially within the VM. Parallelism comes from:

1. **Multiple QEMU instances.** The host launches N QEMU instances in parallel, each running a partition of the test suite. The system test framework already handles port allocation and independent instances.

2. **Partitioned manifests.** The test manifest is split into N chunks:
   ```
   manifest-0.txt  (tests 1-100)
   manifest-1.txt  (tests 101-200)
   ...
   ```
   Each QEMU instance receives a different manifest via a different disk image (or by passing the partition number as a kernel command-line argument).

3. **Nextest parallelism.** Since each system test is a separate tokio test, nextest can run multiple test functions in parallel, each booting its own QEMU.

### 3.4 Timeout Handling

Tests can hang if they trigger a kernel deadlock or wait for unimplemented functionality. Timeouts must be enforced at two levels:

1. **Inside the VM (compliance-runner):** fork the test, then `waitpid()` with `WNOHANG` in a poll loop, checking `clock_gettime()` for a per-test timeout (e.g., 30 seconds). If the test exceeds the timeout, `kill(child_pid, SIGKILL)` and report `TIMEOUT`.

2. **On the host (system test):** The `ReadAsserter` already has a configurable timeout (default 30s). If no output arrives for 60 seconds, the host kills QEMU and marks all remaining tests as `TIMEOUT`.

---

## 4. CI Integration

### 4.1 Test Tiers

| Tier | What | When | Duration | Purpose |
|------|------|------|----------|---------|
| **Tier 0** | Existing system tests | Every commit / PR | ~2 min | Prevent regressions in current features |
| **Tier 1** | libc-test (full) | Every commit / PR | ~5 min | Core libc compliance |
| **Tier 2** | LTP "smoke" subset | Every PR | ~15 min | Key syscall coverage |
| **Tier 3** | LTP full syscalls | Nightly | ~2 hours | Comprehensive syscall compliance |
| **Tier 4** | LTP full + kselftest + stress-ng | Weekly | ~6 hours | Full compliance measurement |

### 4.2 LTP Smoke Subset (Tier 2)

Select ~100 tests covering the most important syscalls. These represent the "core POSIX surface" that most programs need:

```
# Process management
fork01 fork02 clone01 clone02 execve01 execve02 exit01 wait401 waitpid01
# File I/O
read01 read02 write01 write02 open01 close01 lseek01 dup01 dup201
# File system
mkdir01 rmdir01 unlink01 link01 stat01 fstat01 getcwd01
# Memory
brk01 mmap01 mmap02 munmap01 mprotect01
# Signals
kill01 kill02 sigaction01 sigprocmask01 sigsuspend01
# Time
clock_gettime01 nanosleep01 timer_create01
# IPC
pipe01 pipe02
# Network
socket01 bind01 connect01 sendto01 recvfrom01
# Misc
getpid01 getppid01 getuid01 umask01 chdir01
```

This subset can be maintained as a file: `compliance/ltp-smoke.txt`.

### 4.3 CI Workflow

```yaml
# .github/workflows/compliance.yml
name: compliance

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: '0 2 * * *'  # Nightly at 2 AM

jobs:
  tier1-libc-test:
    runs-on: [self-hosted, nix]
    steps:
      - uses: actions/checkout@v6
      - uses: ./.github/actions/setup
      - run: just build-compliance-image libc-test
      - run: just run-compliance libc-test
      - uses: actions/upload-artifact@v4
        with:
          name: libc-test-results
          path: compliance/results-libc-test.json

  tier2-ltp-smoke:
    runs-on: [self-hosted, nix]
    if: github.event_name == 'pull_request' || github.event_name == 'push'
    steps:
      - uses: actions/checkout@v6
      - uses: ./.github/actions/setup
      - run: just build-compliance-image ltp-smoke
      - run: just run-compliance ltp-smoke

  tier3-ltp-full:
    runs-on: [self-hosted, nix]
    if: github.event_name == 'schedule'
    steps:
      - uses: actions/checkout@v6
      - uses: ./.github/actions/setup
      - run: just build-compliance-image ltp-syscalls
      - run: just run-compliance ltp-syscalls
```

### 4.4 Regression Prevention

Once a compliance test starts passing, it must never regress. The CI should:

1. Load the previous `results.json` (from main branch or a committed baseline).
2. Compare with the new run.
3. **Fail the PR** if any previously-passing test now fails.
4. **Celebrate** if new tests pass (log the improvement).

This is the "ratchet" pattern: progress is monotonic.

---

## 5. Recommended Order of Integration

### Phase 1: libc-test (Weeks 1-2)

**Why first:**
- Smallest suite (~300 tests)
- Directly tests the musl+kernel interface that every program uses
- Simple build, simple runner, simple output format
- Each test is a standalone static binary
- Fastest feedback loop for development

**Concrete steps:**
1. Cross-compile libc-test with `riscv64-unknown-linux-musl-gcc -static`
2. Build an ext2 image containing the test binaries
3. Write `compliance-runner` (can be a simple C program initially)
4. Write a system test that boots QEMU, runs the suite, parses output
5. Generate `results.json` and `DASHBOARD.md`
6. Add Tier 1 CI job

### Phase 2: LTP Syscalls (Weeks 3-6)

**Why second:**
- The "gold standard" for Linux kernel compliance
- The syscalls subset (`testcases/kernel/syscalls/`) directly maps to Solaya's implementation work
- Each failing test points to a specific missing or broken syscall
- The test-per-syscall granularity makes it perfect for AI-driven development

**Concrete steps:**
1. Cross-compile LTP with static musl (expect some compilation failures; maintain a skiplist)
2. Create a large ext2 image with the compiled syscall tests
3. Start with the smoke subset (~100 tests)
4. Expand to full syscalls as the kernel matures
5. Add Tier 2 and Tier 3 CI jobs

### Phase 3: kselftest Subset (Weeks 7-8)

**Why third:**
- Complements LTP for specific subsystems (signals, timers, futex)
- Tests edge cases that LTP may not cover
- Useful for hardening areas that are already partially working

**Concrete steps:**
1. Cross-compile selected kselftest directories (`signal/`, `timers/`, `futex/`)
2. Add to the compliance disk image
3. Extend the runner to handle TAP output format

### Phase 4: stress-ng (Weeks 9-10)

**Why fourth:**
- Only useful after basic correctness is established
- Tests stability, not correctness
- Requires a certain baseline of working syscalls

**Concrete steps:**
1. Cross-compile stress-ng statically
2. Add to disk image as a single binary
3. Write targeted stress tests: `stress-ng --fork 4 -t 30`, `stress-ng --pipe 4 -t 30`, etc.
4. Add as a separate CI tier or integrate into the weekly run

### Phase 5: syzkaller (Future)

Defer until >50% LTP syscall compliance. See section 1.5 for prerequisites.

---

## 6. Automation for AI Agents

### 6.1 The Core Loop

AI agents implement Linux compatibility by following a simple loop:

```
1. Run compliance suite
2. Pick the next failing test
3. Diagnose why it fails (missing syscall? wrong errno? incomplete implementation?)
4. Implement the fix
5. Verify the test passes
6. Run regression check (no previously-passing tests broke)
7. Commit
8. Goto 1
```

### 6.2 Test Selection Strategy

Not all failing tests are equal. The agent should prioritize:

1. **Tests that fail with "unimplemented syscall"** -- these have the clearest fix (implement the syscall).
2. **Tests for syscalls that are partially implemented** -- the syscall exists but returns wrong results.
3. **Tests that many other tests depend on** -- fixing `clone` with `CLONE_THREAD` will unblock all threading tests.
4. **Tests that are close to passing** -- a test that runs for 5 seconds before failing is closer than one that crashes immediately.

### 6.3 Machine-Readable Failure Diagnosis

The compliance runner should capture enough information for an agent to diagnose failures without manual intervention:

```
RESULT libc-test src/functional/pthread FAIL exit=139 signal=SIGSEGV elapsed=12ms
RESULT libc-test src/functional/fork FAIL exit=1 signal=none elapsed=45ms
RESULT ltp-syscalls fork01 PASS exit=0 signal=none elapsed=120ms
RESULT ltp-syscalls clone02 FAIL exit=1 signal=none elapsed=85ms
  OUTPUT: clone02.c:78: TFAIL: clone(CLONE_NEWNS) failed: ENOSYS
```

From this, an agent can determine:
- `pthread` test crashed with SIGSEGV -- likely an unimplemented syscall triggered a null pointer or bad memory access
- `fork` test ran but returned failure -- partial implementation, wrong result
- `clone02` explicitly says `CLONE_NEWNS` returned `ENOSYS` -- the kernel does not support namespace cloning

### 6.4 Agent Workflow Integration

The MCP server (`mcp-server/`) should expose compliance tools:

| Tool | Description |
|------|-------------|
| `run_compliance_suite` | Run a compliance suite (libc-test, ltp-smoke, ltp-full) |
| `get_failing_tests` | Return list of failing tests sorted by priority |
| `get_test_output` | Get detailed output for a specific failing test |
| `get_compliance_summary` | Get pass/fail/skip counts |

This lets an agent say: "Run ltp-smoke, show me the next 5 failing tests, let me look at the output of fork03, I see it needs waitpid -- let me implement that."

### 6.5 Dependency Graph

Some syscalls are prerequisites for others. The agent should know:

```
clone (CLONE_THREAD) -> all pthread tests
pipe2 -> shell pipelines, subprocess communication
mmap (MAP_SHARED) -> shared memory, file mapping tests
rt_sigaction -> signal handling tests
futex -> all threading tests (musl's pthread uses futex)
```

Maintain this as a simple dependency file (`compliance/syscall-deps.txt`) that the agent can consult to choose high-impact work.

### 6.6 Commit Discipline

When an AI agent implements a missing feature to fix a compliance test:

1. **Commit the feature** with a message like: `feat: implement clone3 syscall for thread creation`
2. **Include the test name** in the commit body: `Fixes: ltp/clone301, libc-test/src/functional/pthread`
3. **Update the compliance baseline** if running compliance in CI
4. **Do not implement multiple unrelated features** in one commit -- one syscall per commit makes bisection possible

### 6.7 Tracking Velocity

Track the number of passing tests over time. Plot `pass_count` by date. This gives a clear velocity metric:

```
2026-03-15: libc-test 42/290 (14%), ltp-syscalls 80/1200 (6%)
2026-03-22: libc-test 78/290 (27%), ltp-syscalls 145/1200 (12%)
2026-03-29: libc-test 112/290 (39%), ltp-syscalls 210/1200 (17%)
...
```

The JSON results file makes this trivial to compute. Store historical snapshots in `compliance/history/` or a simple CSV.

---

## Appendix A: Solaya's Current Syscall Coverage vs. LTP

Solaya implements ~74 syscalls. LTP has test directories for ~376 syscalls. Of Solaya's current syscalls, the following have direct LTP test coverage:

| Solaya syscall | LTP tests exist? | Priority |
|----------------|-------------------|----------|
| read, write, readv, writev | Yes (multiple) | Already testable |
| openat, close, lseek | Yes | Already testable |
| fork (via clone) | Yes | Already testable |
| execve | Yes | Already testable |
| mmap, munmap, mprotect | Yes | Already testable |
| pipe2 | Yes | Already testable |
| fstat, newfstatat, statx | Yes | Already testable |
| getdents64 | Yes | Already testable |
| brk | Yes | Already testable |
| clock_gettime, nanosleep | Yes | Already testable |
| rt_sigaction, rt_sigprocmask | Yes | Already testable |
| kill, tgkill | Yes | Already testable |
| socket, bind, connect, sendto, recvfrom | Yes | Already testable |
| wait4 | Yes (via waitpid tests) | Already testable |
| fcntl | Yes | Already testable |
| dup3 | Yes | Already testable |
| futex | Yes | Already testable |
| mkdirat, unlinkat | Yes | Already testable |
| chdir, getcwd | Yes | Already testable |
| getpid, getppid, getuid, etc. | Yes | Already testable |
| ppoll | Yes | Already testable |
| splice | Yes | Already testable |

This means a significant portion of existing LTP syscall tests should be runnable today, giving immediate feedback on correctness.

## Appendix B: Cross-Compilation Quick Reference

All test suites use the same toolchain. The `riscv64-unknown-linux-musl` toolchain is already available in Solaya's Nix environment (used for userspace builds).

```bash
# Common environment variables
export CC=riscv64-unknown-linux-musl-gcc
export CXX=riscv64-unknown-linux-musl-g++
export AR=riscv64-unknown-linux-musl-ar
export RANLIB=riscv64-unknown-linux-musl-ranlib
export STRIP=riscv64-unknown-linux-musl-strip
export CFLAGS="-static"
export LDFLAGS="-static"

# libc-test
cd libc-test && cp config.mak.def config.mak
# Edit config.mak with CC and CFLAGS above
make

# LTP
cd ltp && make autotools
./configure --host=riscv64-linux-musl --prefix=/opt/ltp \
    CFLAGS="-static -D_GNU_SOURCE" LDFLAGS="-static -pthread"
make -j$(nproc) && make install DESTDIR=./install

# kselftest (from Linux kernel tree)
make -C tools/testing/selftests ARCH=riscv \
    CROSS_COMPILE=riscv64-unknown-linux-musl- \
    TARGETS="signal timers futex" install

# stress-ng
cd stress-ng && STATIC=1 make -j$(nproc)
```

## Appendix C: Disk Image Creation

```bash
#!/bin/bash
# scripts/build-compliance-image.sh
set -euo pipefail

SUITE="$1"
IMAGE="compliance/images/${SUITE}.img"
STAGING=$(mktemp -d)

case "$SUITE" in
    libc-test)
        cp -r compliance/build/libc-test/src/functional/*-static "$STAGING/"
        cp -r compliance/build/libc-test/src/regression/*-static "$STAGING/"
        cp -r compliance/build/libc-test/src/math/*-static "$STAGING/"
        SIZE_MB=32
        ;;
    ltp-smoke)
        while read -r name cmd; do
            binary="compliance/build/ltp/testcases/bin/$cmd"
            [ -f "$binary" ] && cp "$binary" "$STAGING/"
        done < compliance/ltp-smoke.txt
        SIZE_MB=64
        ;;
    ltp-syscalls)
        cp -r compliance/build/ltp/testcases/bin/* "$STAGING/"
        SIZE_MB=512
        ;;
esac

# Generate manifest
ls "$STAGING/" > "$STAGING/manifest.txt"

# Create ext2 image
dd if=/dev/zero of="$IMAGE" bs=1M count="$SIZE_MB"
mkfs.ext2 -d "$STAGING" -F "$IMAGE"

rm -rf "$STAGING"
echo "Created $IMAGE"
```
