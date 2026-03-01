import signal

from mcp.server.fastmcp import FastMCP
from .gdb_session import GDBSession, DEFAULT_KERNEL_PATH

mcp = FastMCP("gdb")
session = GDBSession()


class ToolTimeout(Exception):
    pass


def _with_timeout(func, timeout_sec):
    def handler(signum, frame):
        raise ToolTimeout(f"Timed out after {timeout_sec}s")
    old = signal.signal(signal.SIGALRM, handler)
    signal.alarm(timeout_sec)
    try:
        return func()
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old)


def _format_responses(responses: list[dict]) -> str:
    lines = []
    for r in responses:
        if r.get("type") == "console":
            payload = r.get("payload", "")
            if payload:
                lines.append(payload)
        elif r.get("type") == "result":
            payload = r.get("payload")
            if payload:
                lines.append(str(payload))
        elif r.get("type") == "notify":
            msg = r.get("message", "")
            payload = r.get("payload", {})
            if msg:
                lines.append(f"[{msg}] {payload}")
    return "".join(lines) if lines else "OK"


def _format_error(responses: list[dict]) -> str | None:
    for r in responses:
        if r.get("message") == "error":
            payload = r.get("payload", {})
            msg = payload.get("msg", str(payload))
            return msg
    return None


def _run_mi(command: str, timeout_sec: int = 30) -> str:
    def inner():
        responses = session.execute_mi(command, timeout_sec=timeout_sec)
        err = _format_error(responses)
        if err:
            return f"Error: {err}"
        return _format_responses(responses)
    return _with_timeout(inner, timeout_sec + 5)


def _run_cli(command: str, timeout_sec: int = 30) -> str:
    def inner():
        responses = session.execute_cli(command, timeout_sec=timeout_sec)
        err = _format_error(responses)
        if err:
            return f"Error: {err}"
        return _format_responses(responses)
    return _with_timeout(inner, timeout_sec + 5)


@mcp.tool()
def gdb_connect(
    port: int | None = None,
    kernel_path: str = DEFAULT_KERNEL_PATH,
    gdb_path: str = "gdb",
) -> str:
    """Start GDB, load kernel symbols, and connect to QEMU.
    Reads port from .gdb-port if not specified."""
    if session.connected:
        return "Already connected. Use gdb_disconnect first."
    if port is None:
        port = session.read_gdb_port()
        if port is None:
            return "Error: No port specified and .gdb-port not found. Is QEMU running?"

    def inner():
        try:
            startup = session.start(gdb_path)
            responses = session.connect_remote(port, kernel_path)
            all_responses = startup + responses
        except Exception as e:
            session.stop()
            return f"Error starting GDB: {e}"

        err = _format_error(all_responses)
        if err:
            session.stop()
            return f"Error connecting: {err}"

        return f"Connected to QEMU on port {port} with kernel {kernel_path}"

    try:
        return _with_timeout(inner, 30)
    except ToolTimeout:
        session.stop()
        return "Error: gdb_connect timed out after 30s"


@mcp.tool()
def gdb_disconnect() -> str:
    """Stop GDB session and clean up."""
    session.stop()
    return "Disconnected."


@mcp.tool()
def gdb_backtrace(full: bool = False) -> str:
    """Get stack trace. Set full=True to include local variables."""
    if full:
        return _run_cli("bt full")
    return _run_mi("-stack-list-frames")


@mcp.tool()
def gdb_breakpoint(location: str, hardware: bool = True) -> str:
    """Set a breakpoint. Uses hardware breakpoints by default (reliable on RISC-V).
    Location can be function name, file:line, or address (*0x...)."""
    if hardware:
        return _run_cli(f"hbreak {location}")
    return _run_mi(f"-break-insert {location}")


@mcp.tool()
def gdb_continue() -> str:
    """Resume execution until breakpoint or signal."""
    return _run_mi("-exec-continue", timeout_sec=60)


@mcp.tool()
def gdb_step() -> str:
    """Step into next source line."""
    return _run_mi("-exec-step")


@mcp.tool()
def gdb_next() -> str:
    """Step over next source line."""
    return _run_mi("-exec-next")


@mcp.tool()
def gdb_print(expression: str) -> str:
    """Evaluate an expression (variable, register, memory dereference, etc.)."""
    escaped = expression.replace('"', '\\"')
    return _run_mi(f"-data-evaluate-expression \"{escaped}\"")


@mcp.tool()
def gdb_execute(command: str, timeout_sec: int = 30) -> str:
    """Run an arbitrary GDB CLI command. Escape hatch for anything not covered by other tools."""
    return _run_cli(command, timeout_sec=timeout_sec)


@mcp.tool()
def gdb_registers() -> str:
    """Read all CPU registers."""
    return _run_mi("-data-list-register-values x")


@mcp.tool()
def gdb_locals(frame: int | None = None) -> str:
    """Get local variables in the current or specified stack frame."""
    if frame is not None:
        responses = session.execute_mi(f"-stack-select-frame {frame}")
        err = _format_error(responses)
        if err:
            return f"Error selecting frame {frame}: {err}"
    return _run_mi("-stack-list-locals --all-values")


@mcp.tool()
def gdb_examine(address: str, count: int = 16, unit: str = "g", fmt: str = "x") -> str:
    """Examine memory. Default: 16 giant words (8 bytes each) in hex. Common units: b=1, h=2, w=4, g=8."""
    return _run_cli(f"x/{count}{fmt}{unit} {address}")


@mcp.tool()
def gdb_breakpoint_list() -> str:
    """List all breakpoints with status and hit counts."""
    return _run_mi("-break-list")


@mcp.tool()
def gdb_breakpoint_delete(number: int) -> str:
    """Delete a breakpoint by its number."""
    return _run_mi(f"-break-delete {number}")


@mcp.tool()
def gdb_interrupt() -> str:
    """Pause the running kernel by sending SIGINT to GDB."""
    session.interrupt()
    try:
        responses = session._require_gdb().get_gdb_response(timeout_sec=5)
        output = _format_responses(responses)
        if output and output != "OK":
            return f"Interrupted. {output}"
        return "Interrupted."
    except Exception:
        return "Interrupt sent (no stop response received)."


@mcp.tool()
def gdb_finish() -> str:
    """Run until the current function returns."""
    return _run_mi("-exec-finish", timeout_sec=60)


@mcp.tool()
def gdb_threads() -> str:
    """List all threads/CPU harts."""
    return _run_mi("-thread-info")


@mcp.tool()
def gdb_select_thread(thread_id: int) -> str:
    """Switch to a different thread/hart."""
    return _run_mi(f"-thread-select {thread_id}")


@mcp.tool()
def gdb_frame(frame_number: int) -> str:
    """Select a stack frame for inspection."""
    responses = session.execute_mi(f"-stack-select-frame {frame_number}")
    err = _format_error(responses)
    if err:
        return f"Error: {err}"
    return _run_mi("-stack-info-frame")


@mcp.tool()
def gdb_diagnose() -> str:
    """One-shot diagnostic: interrupt kernel, list all threads, get backtrace for each.
    Returns a combined report useful for diagnosing deadlocks and hangs."""
    def inner():
        parts = []

        session.interrupt()
        try:
            responses = session._require_gdb().get_gdb_response(timeout_sec=5)
            stop_info = _format_responses(responses)
            if stop_info and stop_info != "OK":
                parts.append(f"Stop reason: {stop_info}")
        except Exception:
            parts.append("Interrupt sent (no stop response received)")

        thread_responses = session.execute_mi("-thread-info", timeout_sec=10)
        err = _format_error(thread_responses)
        if err:
            parts.append(f"Thread list error: {err}")
            return "\n\n".join(parts)

        thread_ids = []
        for r in thread_responses:
            if r.get("type") == "result":
                payload = r.get("payload", {})
                threads = payload.get("threads", [])
                for t in threads:
                    tid = t.get("id")
                    if tid:
                        thread_ids.append(tid)

        parts.append(f"Threads: {len(thread_ids)}")

        for tid in thread_ids:
            select_responses = session.execute_mi(f"-thread-select {tid}", timeout_sec=5)
            select_err = _format_error(select_responses)
            if select_err:
                parts.append(f"--- Thread {tid} ---\nError selecting thread: {select_err}")
                continue
            bt_responses = session.execute_cli("bt", timeout_sec=10)
            bt_err = _format_error(bt_responses)
            bt_text = f"Error: {bt_err}" if bt_err else _format_responses(bt_responses)
            parts.append(f"--- Thread {tid} ---\n{bt_text}")

        return "\n\n".join(parts)

    try:
        return _with_timeout(inner, 60)
    except ToolTimeout:
        return "Error: gdb_diagnose timed out after 60s"
